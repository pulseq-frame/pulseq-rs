use std::{collections::HashMap, hash::Hash};

use super::*;
use crate::{
    error::ParseError,
    parse_file::{BlockDuration, Section, Version},
};

macro_rules! extract {
    ($sections:expr, $variant:ident) => {{
        let mut extracted = Vec::new();
        while let Some(index) = $sections
            .iter()
            .position(|x| matches!(x, Section::$variant(_)))
        {
            match $sections.swap_remove(index) {
                Section::$variant(section_data) => extracted.push(section_data),
                _ => unreachable!(),
            }
        }
        extracted
    }};
}

fn convert_sec<Data, Key: Eq + Hash, Val, F: Fn(Data) -> (Key, Val)>(
    sec_data: Vec<Vec<Data>>,
    f: F,
) -> Result<HashMap<Key, Val>, ParseError> {
    let tmp: Vec<_> = sec_data.into_iter().flatten().map(f).collect();
    let count = tmp.len();
    let tmp: HashMap<_, _> = tmp.into_iter().collect();

    if tmp.len() < count {
        Err(ParseError::Generic)
    } else {
        Ok(tmp)
    }
}

pub fn from_raw(mut sections: Vec<Section>) -> Result<Sequence, ParseError> {
    // Destructure into single section or return error
    let [version]: [Version; 1] = extract!(sections, Version)
        .try_into()
        .map_err(|_| ParseError::Generic)?;

    let defs: Vec<_> = extract!(sections, Definitions)
        .into_iter()
        .flatten()
        .collect();
    let (name, fov, definitions, time_raster) = convert_defs(&version, defs)?;

    let shapes = convert_sec(extract!(sections, Shapes), |shape| {
        (shape.id, Arc::new(Shape(shape.samples)))
    })?;

    let delays = convert_sec(extract!(sections, Delays), |delay| (delay.id, delay.delay))?;
    let adcs = convert_sec(extract!(sections, Adcs), |adc| {
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
    })?;
    let rfs = convert_sec(extract!(sections, Rfs), |rf| {
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
    })?;
    let mut gradients = convert_sec(extract!(sections, Gradients), |grad| {
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
    })?;
    let traps = convert_sec(extract!(sections, Traps), |trap| {
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
    })?;

    // Gradients and Traps share keys
    let count = gradients.len() + traps.len();
    gradients.extend(traps.into_iter());
    if gradients.len() < count {
        return Err(ParseError::Generic);
    }

    let blocks = extract!(sections, Blocks)
        .into_iter()
        .flatten()
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

fn convert_defs(
    version: &Version,
    defs: Vec<(String, String)>,
) -> Result<
    (
        Option<String>,
        Option<(f32, f32, f32)>,
        HashMap<String, String>,
        TimeRaster,
    ),
    ParseError,
> {
    let def_count = defs.len();
    let mut defs: HashMap<_, _> = defs.into_iter().collect();
    if defs.len() < def_count {
        // Duplicated key
        return Err(ParseError::Generic);
    }

    // Before 1.4, there is no spec on what's inside of a definition, so we
    // just directly return. Raster times are not exported by older exporters,
    // so we don't need to waste time trying to parse them.
    if !matches!(
        version,
        Version {
            major: 1,
            minor: 4,
            ..
        }
    ) {
        return Ok((None, None, defs, TimeRaster::default()));
    }

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

    Ok((name, fov, defs, time_raster))
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
