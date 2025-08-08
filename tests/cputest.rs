#[cfg(feature = "cputest")]
mod test {
    use serde::Deserialize;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use tabled::builder::Builder;
    use tabled::settings::Alignment;
    use tabled::settings::Style;

    use nestacean::error::Error;
    use nestacean::error::Result;
    use nestacean::prelude::*;

    #[derive(Deserialize)]
    struct State {
        pc: u16,
        s: u8,
        a: u8,
        x: u8,
        y: u8,
        p: u8,
        ram: Vec<(u16, u8)>,
    }

    #[derive(Deserialize)]
    struct Test {
        initial: State,
        #[serde(rename(deserialize = "final"))]
        end: State,
        cycles: Vec<(u16, u8, String)>,
    }

    struct HashBus {
        memory: RefCell<HashMap<u16, u8>>,
    }

    impl HashBus {
        pub fn new(values: &mut dyn Iterator<Item = &(u16, u8)>) -> Self {
            Self {
                memory: RefCell::new(values.cloned().collect()),
            }
        }

        pub fn equals(&self, values: &mut dyn Iterator<Item = &(u16, u8)>) -> bool {
            *self.memory.borrow() == values.cloned().collect()
        }
    }

    impl Bus for HashBus {
        fn read(&self, addr: u16) -> Result<u8> {
            self.memory
                .borrow()
                .get(&addr)
                .cloned()
                .ok_or(Error::UnresolvedAddress(addr))
        }

        fn write(&self, addr: u16, value: u8) -> Result<()> {
            self.memory.borrow_mut().insert(addr, value);
            Ok(())
        }
    }

    pub fn test(name: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(format!("tests/nes6502/{name}.json"))?;
        let tests: Vec<Test> = serde_json::from_str(&contents)?;

        for test in &tests {
            let bus = HashBus::new(&mut test.initial.ram.iter());

            let mut cpu = Cpu::new(&bus);
            cpu.pc = test.initial.pc;
            cpu.s = test.initial.s;
            cpu.a = test.initial.a;
            cpu.x = test.initial.x;
            cpu.y = test.initial.y;
            cpu.p = test.initial.p;

            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let cycles = cpu.step().unwrap_or_else(|e| panic!("{}", e));
                assert_eq!(usize::from(cycles), test.cycles.len());

                assert_eq!(cpu.pc, test.end.pc);
                assert_eq!(cpu.s, test.end.s);
                assert_eq!(cpu.a, test.end.a);
                assert_eq!(cpu.x, test.end.x);
                assert_eq!(cpu.y, test.end.y);
                assert_eq!(cpu.p, test.end.p);
                assert!(bus.equals(&mut test.end.ram.iter()));
            })) {
                macro_rules! row {
                ($name:literal, $fmt:literal, $($expr:expr),* $(,)?) => {[$name, $(&format!($fmt, $expr)),*]}
            }
                let mut builder = Builder::new();
                builder.push_record(["Register", "Initial", "Current", "Expected"]);
                builder.push_record(row!["PC", "0x{:04X}", test.initial.pc, cpu.pc, test.end.pc]);
                builder.push_record(row!["S", "0x{:02X}", test.initial.s, cpu.s, test.end.s]);
                builder.push_record(row!["A", "0x{:02X}", test.initial.a, cpu.a, test.end.a]);
                builder.push_record(row!["X", "0x{:02X}", test.initial.x, cpu.x, test.end.x]);
                builder.push_record(row!["Y", "0x{:02X}", test.initial.y, cpu.y, test.end.y]);
                builder.push_record(row!["P", "0b{:08b}", test.initial.p, cpu.p, test.end.p]);

                let mut table = builder.build();
                table.with(Style::modern_rounded());
                println!("{}", table);

                let mut builder = Builder::new();
                builder.push_record(["Address", "Initial", "Current", "Expected"]);

                let initial = test.initial.ram.iter().cloned().collect::<HashMap<_, _>>();
                let end = test.end.ram.iter().cloned().collect::<HashMap<_, _>>();
                let mem = bus.memory.borrow();

                let mut addresses = mem
                    .keys()
                    .chain(initial.keys())
                    .chain(end.keys())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                addresses.sort();

                let mut last = None::<u16>;

                for addr in &addresses {
                    if let Some(value) = last
                        && value + 1 != *addr
                    {
                        builder.push_record(["⋮", "⋮", "⋮", "⋮"]);
                    }

                    last.replace(*addr);

                    builder.push_record([
                        format!("0x{:04X}", addr),
                        initial
                            .get(addr)
                            .map_or(String::new(), |x| format!("0x{:02X}", x)),
                        end.get(addr)
                            .map_or(String::new(), |x| format!("0x{:02X}", x)),
                        mem.get(addr)
                            .map_or(String::new(), |x| format!("0x{:02X}", x)),
                    ])
                }

                let mut table = builder.build();
                table.with(Style::rounded()).with(Alignment::center());
                println!("{}", table);

                std::panic::resume_unwind(e);
            }

            // TODO: Cycle length check.
        }

        Ok(())
    }
}

macro_rules! test {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = test::test($name) {
                panic!("{}", e)
            }
        }
    };
}

#[cfg(feature = "cputest")]
include!(concat!(env!("OUT_DIR"), "/cputest.rs"));
