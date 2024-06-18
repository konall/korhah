use std::io::BufRead;

struct InputEvent {
    line: String,
}

fn main() {
    let mut system = korhah::System::default();

    // # lines
    let lines = system
        .create(|_, _| 0)
        .expect("no cancelling listeners registered");

    system.listen(None, move |s, _: &InputEvent, _, _| {
        _ = s.update(lines, |v| *v += 1);
    });

    // # characters
    let chars = system
        .create(move |_, _| 0)
        .expect("no cancelling listeners registered");

    system.listen(None, move |s, e: &InputEvent, _, _| {
        _ = s.update(chars, |v| *v += e.line.chars().count());
    });

    // average # characters per line
    let average = system
        .create(move |s, _| {
            let lines = s
                .read(lines, |v| *v)
                .expect("no cancelling listeners registered")
                .expect("`lines` exists");
            let chars = s
                .read(chars, |v| *v)
                .expect("no cancelling listeners registered")
                .expect("`chars` exists");
            chars as f32 / lines as f32
        })
        .expect("no cancelling listeners registered");

    for line in std::io::stdin().lock().lines().flatten() {
        match line.as_str() {
            "@exit" => {
                println!("Exiting...");
                break;
            }
            "@lines" => {
                println!(
                    "-> {} lines read",
                    system
                        .read(lines, |v| *v)
                        .expect("no cancelling listeners registered")
                        .expect("`lines` exists")
                );
            }
            "@chars" => {
                println!(
                    "-> {} characters read",
                    system
                        .read(chars, |v| *v)
                        .expect("no cancelling listeners registered")
                        .expect("`chars` exists")
                );
            }
            "@avg" => {
                println!(
                    "-> average of {} characters read per line",
                    system
                        .read(average, |v| *v)
                        .expect("no cancelling listeners registered")
                        .expect("`average` exists")
                );
            }
            _ => {
                _ = system.emit(None, &InputEvent { line });
            }
        }
    }
}
