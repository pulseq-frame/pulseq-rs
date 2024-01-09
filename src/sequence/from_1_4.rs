use std::collections::HashMap;

use crate::parsers::{helpers::ParseError, Section};

use super::*;

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

impl Sequence {
    pub fn from_1_4(mut sections: Vec<Section>) -> Result<Self, ParseError> {
        // TODO: throw an error if definitions are missing in a 1.4 file
        let (name, fov, definitions, time_raster) = if sections
            .iter()
            .filter(|&s| matches!(s, Section::Definitions(_)))
            .count()
            > 0
        {
            let defs = extract!(sections, Definitions);
            (
                defs.name,
                defs.fov,
                defs.rest,
                TimeRaster {
                    grad: defs.grad_raster,
                    rf: defs.rf_raster,
                    adc: defs.adc_raster,
                    block: defs.block_dur_raster,
                },
            )
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

        Ok(Self {
            name,
            fov,
            definitions,
            time_raster,
            blocks,
        })
    }
}

fn convert_block(
    block: crate::parsers::Block,
    rfs: &HashMap<u32, Arc<Rf>>,
    gradients: &HashMap<u32, Arc<Gradient>>,
    adcs: &HashMap<u32, Arc<Adc>>,
    delays: &HashMap<u32, f32>,
    time_raster: &TimeRaster,
) -> Result<Block, ParseError> {
    Ok(match block {
        crate::parsers::Block::V131 {
            id,
            delay,
            rf,
            gx,
            gy,
            gz,
            adc,
            ext: _,
        } => {
            let rf = (rf != 0)
                .then(|| rfs.get(&rf).cloned().ok_or(ParseError::Generic))
                .transpose()?;
            let gx = (gx != 0)
                .then(|| gradients.get(&gx).cloned().ok_or(ParseError::Generic))
                .transpose()?;
            let gy = (gy != 0)
                .then(|| gradients.get(&gy).cloned().ok_or(ParseError::Generic))
                .transpose()?;
            let gz = (gz != 0)
                .then(|| gradients.get(&gz).cloned().ok_or(ParseError::Generic))
                .transpose()?;
            let adc = (adc != 0)
                .then(|| adcs.get(&adc).cloned().ok_or(ParseError::Generic))
                .transpose()?;
            let delay = (delay != 0)
                .then(|| delays.get(&delay).cloned().ok_or(ParseError::Generic))
                .transpose()?;

            let mut duration: f32 = 0.0;
            if let Some(rf) = &rf {
                duration = duration.max(rf.duration(time_raster.rf));
            }
            if let Some(gx) = &gx {
                duration = duration.max(gx.duration(time_raster.grad));
            }
            if let Some(gy) = &gy {
                duration = duration.max(gy.duration(time_raster.grad));
            }
            if let Some(gz) = &gz {
                duration = duration.max(gz.duration(time_raster.grad));
            }
            if let Some(adc) = &adc {
                duration = duration.max(adc.duration());
            }
            if let Some(delay) = delay {
                duration = duration.max(delay);
            }

            Block {
                id,
                duration,
                rf,
                gx,
                gy,
                gz,
                adc,
            }
        }
        crate::parsers::Block::V140 {
            id,
            duration,
            rf,
            gx,
            gy,
            gz,
            adc,
            ext: _,
        } => Block {
            id,
            duration: duration as f32 * time_raster.block,
            rf: (rf != 0).then(|| rfs[&rf].clone()),
            gx: (gx != 0).then(|| gradients[&gx].clone()),
            gy: (gy != 0).then(|| gradients[&gy].clone()),
            gz: (gz != 0).then(|| gradients[&gz].clone()),
            adc: (adc != 0).then(|| adcs[&adc].clone()),
        },
    })
}
