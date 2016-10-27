use std::collections::HashMap;
use std::fmt;

enum RegisterWrapper {
    Pointer{ origin: String, position: usize },
    Raw(u16),
}

pub struct GBRegisterSet {

    registers: HashMap<String, RegisterWrapper>,

}

impl GBRegisterSet {

    pub fn new(set: Vec<&str>) -> GBRegisterSet {
        let mut map = HashMap::new();

        for name in set {
            map.insert(name.to_string(), RegisterWrapper::Raw(0x0 as u16));

            let mut chars = name.chars().rev();

            // right
            map.insert(chars.next().unwrap().to_string(), RegisterWrapper::Pointer{
                origin: name.to_string(),
                position: 0,
            });

            // left
            map.insert(chars.next().unwrap().to_string(), RegisterWrapper::Pointer{
                origin: name.to_string(),
                position: 1,
            });

        }

        GBRegisterSet{ registers: map }
    }

    pub fn get16(&self, reg: &String) -> u16 {

        match self.registers.get(reg) {
            Some(rw) => {
                match rw {
                    &RegisterWrapper::Pointer{ origin: ref og, position: pos } => {
                        self.get16(og) >> (8 * pos)
                    },
                    &RegisterWrapper::Raw(data) => data,
                }
            },
            None => {
                println!("Register not found: {}", reg);
                0
            },
        }

    }

    pub fn get8(&self, reg: &String) -> u8 {

        self.get16(reg) as u8

    }

    pub fn put8(&mut self, reg: &String, data: u16) {

        let mut currentReg = reg.clone();
        let mut targetPos = 0 as usize;
        'search: loop {
            let mut res = self.registers.get_mut(&currentReg);

            match res {
                Some(rw) => {
                    match rw {
                        &mut RegisterWrapper::Pointer{ origin: ref orig, position: pos } => {
                            currentReg = orig.clone();
                            targetPos = pos;
                            continue 'search;
                        },
                        &mut RegisterWrapper::Raw(ref mut reg_data) => {

                            // position(0) => right side
                            // position(1) => left side
                            let mut target_mask = (0xFF00 as u16) >> (8 * targetPos);
                            let mut data_shifted = data << (8 * targetPos);

                            *reg_data &= target_mask;
                            *reg_data |= data_shifted;
                            break 'search;
                        },
                    }
                },
                None => {
                    println!("Register not found: {}", reg);
                },
            };

        };

    }

    pub fn put16(&mut self, reg: &String, data: u16) {

        let mut currentReg = reg.clone();
        let mut targetPos = 0 as usize;
        'search: loop {
            let mut res = self.registers.get_mut(&currentReg);

            match res {
                Some(rw) => {
                    match rw {
                        &mut RegisterWrapper::Pointer{ origin: ref orig, position: pos } => {
                            currentReg = orig.clone();
                            targetPos = pos;
                            continue 'search;
                        },
                        &mut RegisterWrapper::Raw(ref mut reg_data) => {

                            *reg_data = data;
                            break 'search;
                        },
                    }
                },
                None => {
                    println!("Register not found: {}", reg);
                },
            };

        };

    }

}

impl fmt::Debug for GBRegisterSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut regs = "GBRegisterSet {\n".to_string();

        for (key, value) in self.registers.iter() {
            regs += &match value {
                &RegisterWrapper::Pointer{ origin: ref orig, position: pos } => {
                    format!("\t{} = Pointer(o: {}, p: {})\n", key, orig, pos)
                },
                &RegisterWrapper::Raw(d16) => {
                    format!("\t{} = 0x{:04X}\n", key, d16)
                },
            };
        }

        regs += "}\n";
        write!(f, "{}", regs,)
    }
}
