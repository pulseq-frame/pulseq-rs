use std::collections::HashMap;

use super::*;
use crate::{
    error::ParseError,
    parse_file::{BlockDuration, Section, Version},
};

macro_rules! extract {
    ($sections:expr, $variant:ident) => {{
        assert_eq!(
            $sections
                .iter()
                .filter(|sec| matches!(sec, Section::$variant(_)))
                .count(),
            1
        );
        let idx = $sections
            .iter()
            .position(|sec| matches!(sec, Section::$variant(_)))
            .unwrap();
        let tmp = $sections.swap_remove(idx);
        match tmp {
            Section::$variant(ret) => ret,
            _ => unreachable!(),
        }
    }};
}

macro_rules! extract_iter {
    ($sections:expr, $variant:ident) => {{
        assert!(
            $sections
                .iter()
                .filter(|sec| matches!(sec, Section::$variant(_)))
                .count()
                < 2
        );
        if let Some(idx) = $sections
            .iter()
            .position(|sec| matches!(sec, Section::$variant(_)))
        {
            let tmp = $sections.swap_remove(idx);
            match tmp {
                Section::$variant(ret) => ret.into_iter(),
                _ => unreachable!(),
            }
        } else {
            Vec::new().into_iter()
        }
    }};
}

pub fn from_raw(mut sections: Vec<Section>) -> Result<Sequence, ParseError> {
    // NOTE: a panic here is okay since without a version, the file should not even have parsed
    let version = extract!(sections, Version);

    // Check if definitions block exists
    let (name, fov, definitions, time_raster) = if sections
        .iter()
        .filter(|&s| matches!(s, Section::Definitions(_)))
        .count()
        > 0
    {
        let defs = extract!(sections, Definitions);
        let def_count = defs.len();
        let mut defs: HashMap<_, _> = defs.into_iter().collect();
        if defs.len() < def_count {
            // Duplicated key
            return Err(ParseError::Generic);
        }

        // Before version 1.4, nothing about the contents of definitions was specified
        if matches!(
            version,
            Version {
                major: 1,
                minor: 4,
                ..
            }
        ) {
            let time_raster = TimeRaster {
                grad: defs
                    .remove("GradientRasterTime")
                    .ok_or(ParseError::Generic)?
                    .parse()?,
                rf: defs
                    .remove("RadiofrequencyRasterTime")
                    .ok_or(ParseError::Generic)?
                    .parse()?,
                adc: defs
                    .remove("AdcRasterTime")
                    .ok_or(ParseError::Generic)?
                    .parse()?,
                block: defs
                    .remove("BlockDurationRaster")
                    .ok_or(ParseError::Generic)?
                    .parse()?,
            };
            let name = defs.remove("Name");
            let fov = defs.remove("FOV").map(parse_fov).transpose()?;

            (name, fov, defs, time_raster)
        } else {
            (None, None, defs, TimeRaster::default())
        }
    } else {
        (None, None, HashMap::new(), TimeRaster::default())
    };

    // TODO: if some ID exists more than once in the file, we overwrite it.

    let shapes: HashMap<_, _> = extract_iter!(sections, Shapes)
        .map(|shape| (shape.id, Arc::new(Shape(shape.samples))))
        .collect();

    let adcs: HashMap<_, _> = extract_iter!(sections, Adcs)
        .map(|adc| {
            (
                adc.id,
                Arc::new(Adc {
                    num: adc.num,
                    dwell: adc.dwell,
                    delay: adc.delay,
                    freq: adc.freq,
                    phase: adc.phase,
                }),
            )
        })
        .collect();

    let delays: HashMap<_, _> = extract_iter!(sections, Delays)
        .map(|delay| (delay.id, delay.delay))
        .collect();

    let gradients: HashMap<_, _> = extract_iter!(sections, Gradients)
        .map(|grad| {
            (
                grad.id,
                Arc::new(Gradient::Free {
                    amp: grad.amp,
                    shape: shapes[&grad.shape_id].clone(),
                    time: if grad.time_id == 0 {
                        None
                    } else {
                        Some(shapes[&grad.time_id].clone())
                    },
                    delay: grad.delay,
                }),
            )
        })
        .chain(extract_iter!(sections, Traps).map(|trap| {
            (
                trap.id,
                Arc::new(Gradient::Trap {
                    amp: trap.amp,
                    rise: trap.rise,
                    flat: trap.flat,
                    fall: trap.fall,
                    delay: trap.delay,
                }),
            )
        }))
        .collect();

    let rfs: HashMap<_, _> = extract_iter!(sections, Rfs)
        .map(|rf| {
            (
                rf.id,
                Arc::new(Rf {
                    amp: rf.amp,
                    phase: rf.phase,
                    amp_shape: shapes[&rf.mag_id].clone(),
                    phase_shape: shapes[&rf.phase_id].clone(),
                    time_shape: (rf.time_id != 0).then(|| shapes[&rf.time_id].clone()),
                    delay: rf.delay,
                    freq: rf.freq,
                }),
            )
        })
        .collect();

    let blocks = extract_iter!(sections, Blocks)
        .map(|block| convert_block(block, &rfs, &gradients, &adcs, &delays, &time_raster))
        .collect::<Result<Vec<Block>, ParseError>>()?;

    Ok(Sequence {
        name,
        fov,
        definitions,
        time_raster,
        blocks,
    })
}

fn convert_block(
    block: crate::parse_file::Block,
    rfs: &HashMap<u32, Arc<Rf>>,
    gradients: &HashMap<u32, Arc<Gradient>>,
    adcs: &HashMap<u32, Arc<Adc>>,
    delays: &HashMap<u32, f32>,
    time_raster: &TimeRaster,
) -> Result<Block, ParseError> {
    let rf = (block.rf != 0)
        .then(|| rfs.get(&block.rf).cloned().ok_or(ParseError::Generic))
        .transpose()?;
    let gx = (block.gx != 0)
        .then(|| gradients.get(&block.gx).cloned().ok_or(ParseError::Generic))
        .transpose()?;
    let gy = (block.gy != 0)
        .then(|| gradients.get(&block.gy).cloned().ok_or(ParseError::Generic))
        .transpose()?;
    let gz = (block.gz != 0)
        .then(|| gradients.get(&block.gz).cloned().ok_or(ParseError::Generic))
        .transpose()?;
    let adc = (block.adc != 0)
        .then(|| adcs.get(&block.adc).cloned().ok_or(ParseError::Generic))
        .transpose()?;

    let duration = match block.dur {
        BlockDuration::Duration(dur) => dur as f32 * time_raster.block,
        BlockDuration::DelayId(delay) => {
            let delay = (delay != 0)
                .then(|| delays.get(&delay).cloned().ok_or(ParseError::Generic))
                .transpose()?;

            [
                rf.as_ref().map(|rf| rf.duration(time_raster.rf)),
                gx.as_ref().map(|gx| gx.duration(time_raster.grad)),
                gy.as_ref().map(|gy| gy.duration(time_raster.grad)),
                gz.as_ref().map(|gz| gz.duration(time_raster.grad)),
                adc.as_ref().map(|adc| adc.duration()),
                delay,
            ]
            .into_iter()
            .flatten()
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0)
        }
    };

    Ok(Block {
        id: block.id,
        duration,
        rf,
        gx,
        gy,
        gz,
        adc,
    })
}

pub fn parse_fov(s: String) -> Result<(f32, f32, f32), ParseError> {
    let splits: Vec<_> = s.split_whitespace().collect();
    if splits.len() != 3 {
        return Err(ParseError::Generic);
    }
    Ok((splits[0].parse()?, splits[1].parse()?, splits[2].parse()?))
}
