use std::{collections::HashMap, fmt::Display};

// Serialization of the sequence, implemented via the display trait

use super::*;

impl Display for Sequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "METADATA")?;
        writeln!(f, "--------")?;
        writeln!(f, "{}\n", self.metadata)?;

        writeln!(f, "BLOCKS")?;
        writeln!(f, "------")?;
        writeln!(f, "#  ID   RF ( GX,  GY,  GZ) ADC | duration")?;

        let mut rf_refs = RefPrinter::new();
        let mut grad_refs = RefPrinter::new();
        let mut adc_refs = RefPrinter::new();

        for block in &self.blocks {
            block.fmt(f, &mut rf_refs, &mut grad_refs, &mut adc_refs)?;
        }

        let mut shape_refs = RefPrinter::new();

        writeln!(f, "\n\nRFS")?;
        writeln!(f, "---")?;
        writeln!(
            f,
            "#  ID       amp {{ ID}}    phase {{ ID}}    delay     freq"
        )?;
        writeln!(f, "#                [HZ]          [rad]     [ms]    [kHz]")?;
        rf_refs.fmt(f, &mut shape_refs)?;

        writeln!(f, "\n\nGRADIENTS")?;
        writeln!(f, "---------")?;
        writeln!(f, "#  ID  F    delay      amp {{ ID}} {{ TIME_ID}}")?;
        writeln!(
            f,
            "#  ID  T    delay      amp (    rise,     flat,     fall)"
        )?;
        writeln!(
            f,
            "#            [ms]  [kHz/m] (    [ms],     [ms],     [ms])"
        )?;
        grad_refs.fmt(f, &mut shape_refs)?;

        writeln!(f, "\n\nADCS")?;
        writeln!(f, "----")?;
        writeln!(f, "#  ID   num    dwell    delay     freq    phase")?;
        writeln!(f, "#               [us]     [ms]     [Hz]    [rad]")?;
        writeln!(f, "{adc_refs}")?;

        writeln!(f, "\nSHAPES")?;
        writeln!(f, "------")?;
        writeln!(f, "#  ID     num")?;
        write!(f, "{shape_refs}")?;

        Ok(())
    }
}

impl Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            writeln!(f, "name:        '{name}'")?;
        } else {
            writeln!(f, "name:        ?")?;
        }
        if let Some(fov) = &self.fov {
            writeln!(f, "fov:         {fov:?}")?;
        } else {
            writeln!(f, "fov:         ?")?;
        }
        if let Some(grad_raster) = &self.grad_raster {
            writeln!(f, "grad_raster: {grad_raster:?}")?;
        } else {
            writeln!(f, "grad_raster: ?")?;
        }
        if let Some(rf_raster) = &self.rf_raster {
            writeln!(f, "rf_raster:   {rf_raster}")?;
        } else {
            writeln!(f, "rf_raster:   ?")?;
        }

        Ok(())
    }
}

impl Block {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        rf_refs: &mut RefPrinter<Rf>,
        grad_refs: &mut RefPrinter<Gradient>,
        adc_refs: &mut RefPrinter<Adc>,
    ) -> std::fmt::Result {
        write!(f, "[{:4}] ", self.id)?;

        write!(f, "{} ", rf_refs.print_opt(&self.rf))?;
        write!(f, "({}, ", grad_refs.print_opt(&self.gx))?;
        write!(f, "{}, ", grad_refs.print_opt(&self.gy))?;
        write!(f, "{}) ", grad_refs.print_opt(&self.gz))?;
        write!(f, "{} ", adc_refs.print_opt(&self.adc))?;

        writeln!(f, "| {} ms", self.duration * 1e3)
    }
}

struct RefPrinter<T>(HashMap<usize, (Rc<T>, usize)>);

impl<T> RefPrinter<T> {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn print_opt(&mut self, opt_rc: &Option<Rc<T>>) -> String {
        if let Some(rc) = opt_rc {
            let tmp = Rc::as_ptr(&rc) as usize;
            let next_id = self.0.len() + 1;
            let (_, ref id) = self.0.entry(tmp).or_insert((rc.clone(), next_id));
            format!("{id:3}")
        } else {
            "  -".to_owned()
        }
    }

    fn print(&mut self, rc: &Rc<T>) -> String {
        let tmp = Rc::as_ptr(&rc) as usize;
        let next_id = self.0.len() + 1;
        let (_, ref id) = self.0.entry(tmp).or_insert((rc.clone(), next_id));
        format!("{id:3}")
    }
}

impl RefPrinter<Rf> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        shape_refs: &mut RefPrinter<Shape>,
    ) -> std::fmt::Result {
        let mut tmp: Vec<_> = self.0.iter().map(|(_addr, (rc, id))| (rc, *id)).collect();
        tmp.sort_by_key(|(_, id)| *id);

        for (rc, id) in tmp {
            writeln!(
                f,
                "[{id:4}] {:8.3} {{{}}} {:8.3} {{{}}} {:8.3} {:8.3}",
                rc.amp,
                shape_refs.print(&rc.amp_shape),
                rc.phase,
                shape_refs.print(&rc.phase_shape),
                rc.delay * 1e3,
                rc.freq / 1e3
            )?;
        }

        Ok(())
    }
}

impl Display for RefPrinter<Adc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tmp: Vec<_> = self.0.iter().map(|(_addr, (rc, id))| (rc, *id)).collect();
        tmp.sort_by_key(|(_, id)| *id);

        for (rc, id) in tmp {
            writeln!(
                f,
                "[{id:4}] {:4} {:8.3} {:8.3} {:8.3} {:8.3}",
                rc.num,
                rc.dwell * 1e6,
                rc.delay * 1e3,
                rc.freq / 1e3,
                rc.phase,
            )?;
        }

        Ok(())
    }
}

impl Display for RefPrinter<Shape> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tmp: Vec<_> = self.0.iter().map(|(_addr, (rc, id))| (rc, *id)).collect();
        tmp.sort_by_key(|(_, id)| *id);

        for (rc, id) in tmp {
            writeln!(f, "[{id:4}] {:6}", rc.0.len())?;
        }

        Ok(())
    }
}

impl RefPrinter<Gradient> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        shape_refs: &mut RefPrinter<Shape>,
    ) -> std::fmt::Result {
        let mut tmp: Vec<_> = self.0.iter().map(|(_addr, (rc, id))| (rc, *id)).collect();
        tmp.sort_by_key(|(_, id)| *id);

        for (rc, id) in tmp {
            write!(f, "[{id:4}] ")?;
            match rc.as_ref() {
                Gradient::Free {
                    amp,
                    shape,
                    time,
                    delay,
                } => writeln!(
                    f,
                    "F {:8.3} {:8.3} {{{}}} {{{}}}",
                    delay * 1e3,
                    amp / 1e3,
                    shape_refs.print(shape),
                    shape_refs.print_opt(time),
                )?,
                Gradient::Trap {
                    amp,
                    rise,
                    flat,
                    fall,
                    delay,
                } => writeln!(
                    f,
                    "T {:8.3} {:8.3} ({:8.3}, {:8.3}, {:8.3})",
                    delay * 1e3,
                    amp / 1e3,
                    rise / 1e3,
                    flat / 1e3,
                    fall / 1e3
                )?,
            }
        }

        Ok(())
    }
}
