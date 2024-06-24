use std::{
    error::Error,
    io::BufRead,
    sync::{Arc, Mutex},
};

// a demo resource that needs to interact with `korhah` downstream
struct Resource {
    counter: usize,
}

impl Resource {
    fn new(options: Options) -> Self {
        Self {
            counter: options.initial,
        }
    }

    // an example operation on the resource that requires mutable access
    fn modify(&mut self) {
        self.counter += 1;
    }
}

// our troublesome demo resource options that can't be easily moved into closures as
// they don't implement `Clone` / `Copy`
struct Options {
    initial: usize,
}

struct InputEvent;

fn main() -> Result<(), Box<dyn Error>> {
    let mut system = korhah::System::default();

    // approach #1 - Arc + Mutex

    // the options are defined out here to simulate a downstream user passing their own options
    let options = Options { initial: 1 };
    let resource1 = Arc::new(Mutex::new(Resource::new(options)));

    let resource = resource1.clone();
    system.listen(None, move |_, _: &InputEvent, _, _| {
        let mut resource = resource.lock().expect("mutex lock acquired");
        resource.modify();
        println!("-> approach #1 - {}", resource.counter);
    });

    // approach #2 - within `korhah`

    // the options are defined out here to simulate a downstream user passing their own options
    let options = Options { initial: 10 };
    let resource2 = {
        // this commented-out attempt fails, as the `Create` needs to be able to be run an arbitrary
        // number on times (ie: is an `Fn`) in case it reads other variables and thus needs
        // its recipe recomputed whenever one of them changes
        // let resource = system
        //     .create(move |_, _| Resource::new(options))
        //     .expect("no cancelling listeners registered");

        // this attempt succeeds, since we instantiate an "empty" variable in the `Create` recipe,
        // and only initialise it in an `Update` callback into which the options can be safely
        // moved, as it's only run once (ie: is an `FnOnce`)
        let resource = system
            .create(|_, _| Option::<Resource>::None)
            .expect("no cancelling listeners registered");
        _ = system.update(resource, move |v| {
            let resource = Resource::new(options);
            *v = Some(resource);
        });
        resource
    };

    system.listen(None, move |s, _: &InputEvent, _, _| {
        _ = s.update(resource2, |v| v.as_mut().map(|v| v.modify()));
        _ = s.read(resource2, |v| {
            v.as_ref()
                .map(|v| println!("-> approach #2 - {}", v.counter))
        });
    });

    for line in std::io::stdin().lock().lines().flatten() {
        match line.as_str() {
            "@exit" => {
                println!("Exiting...");
                break;
            }
            _ => {
                _ = system.emit(None, &InputEvent);
            }
        }
    }

    Ok(())
}
