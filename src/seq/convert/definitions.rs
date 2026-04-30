use std::collections::HashMap;

use crate::{
    error::{ConversionError, MissingDefinition, ParseFovError},
    raw, seq,
};

/// Simple helper struct to parse definitions into - might be removed after some
/// more refactoring, but as it's contained in this file this is not urgent.
pub struct Defs {
    pub name: Option<String>,
    pub fov: Option<(f64, f64, f64)>,
    pub time_raster: seq::TimeRaster,
    /// lower-cased strings from "RequiredExtensions" definition
    pub required_exts: Vec<String>,
    pub defs: HashMap<String, String>,
}

impl Defs {
    pub fn from_raw(
        version: &raw::Version,
        defs: Vec<(String, String)>,
    ) -> Result<Self, ConversionError> {
        let def_count = defs.len();
        let mut defs: HashMap<_, _> = defs.into_iter().collect();
        if defs.len() < def_count {
            // Duplicated key
            return Err(ConversionError::NonUniqueDefinition);
        }

        // Supported since pulseq 1.5 but earlier versions should not accidentally export this
        let required_exts: Vec<String> = defs
            .remove("RequiredExtensions")
            .unwrap_or(String::new())
            .split_whitespace()
            .map(|s| s.trim().to_lowercase())
            .collect();

        // Before 1.4, there is no spec on what's inside of a definition, so we
        // just directly return. Raster times are not exported by older exporters,
        // so we don't need to waste time trying to parse them.
        if version.major == 1 && version.minor < 4 {
            return Ok(Defs {
                name: None,
                fov: None,
                time_raster: seq::TimeRaster::default(),
                required_exts,
                defs,
            });
        }

        let time_raster = seq::TimeRaster {
            grad: defs
                .remove("GradientRasterTime")
                .ok_or(MissingDefinition::GradientRasterTime)?
                .parse()?,
            rf: defs
                .remove("RadiofrequencyRasterTime")
                .ok_or(MissingDefinition::RadiofrequencyRasterTime)?
                .parse()?,
            adc: defs
                .remove("AdcRasterTime")
                .ok_or(MissingDefinition::AdcRasterTime)?
                .parse()?,
            block: defs
                .remove("BlockDurationRaster")
                .ok_or(MissingDefinition::BlockDurationRaster)?
                .parse()?,
        };
        let name = defs.remove("Name");
        let fov = defs.remove("FOV").map(parse_fov).transpose()?;

        Ok(Defs {
            name,
            fov,
            time_raster,
            required_exts,
            defs,
        })
    }
}

fn parse_fov(s: String) -> Result<(f64, f64, f64), ParseFovError> {
    let splits: Vec<&str> = s.split_whitespace().collect();
    let splits: [&str; 3] = splits
        .try_into()
        .map_err(|vals: Vec<&str>| ParseFovError::WrongValueCount(vals.len()))?;

    Ok((splits[0].parse()?, splits[1].parse()?, splits[2].parse()?))
}
