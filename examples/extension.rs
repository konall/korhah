use std::{collections::HashMap, io::BufRead};

use korhah::{
    events::{Created, Updated, Updating},
    System, Variable, Vote,
};

#[derive(Debug, Default, Clone, Copy)]
struct Item(usize);

struct Update {
    prev: Item,
    next: Item,
}

fn main() {
    let mut system = System::default();

    // track the previous values of variables for which our custom updates are underway
    let prev_values = system
        .create(|_, _| HashMap::<Variable<Item>, Item>::new())
        .expect("no cancelling listeners registered");

    // a flag that is used to circumvent our custom `Update` logic to allow "normal" variable updates to occur whenever
    // our custom `Update` needs undoing 
    let undoing = system
        .create(|_, _| false)
        .expect("no cancelling listeners registered");

    // a test flag that cancels all of our custom `Update` events when set
    let should_cancel = system
        .create(|_, _| false)
        .expect("no cancelling listeners registered");

    // hook into the creation of all `Item` variables in order to define global behaviours
    system.listen(None, move |s, e: &Created<Item>, _, _| {
        let target = e.source;

        s.listen(target, move |s, _: &Updating, _, _| {
            // if we're undoing our custom `Update` event, we need to skip over our custom logic in order to prevent an infinite cycle
            if !s.read(undoing, |v| *v).ok().flatten().unwrap_or_default() {
                // store the previous value of the target, to be retrieved by the `Updated` event later
                let prev = s.read(target, |v| *v).ok().flatten().unwrap_or_default();
                _ = s.update(prev_values, |v| {
                    v.insert(target, prev);
                });
            }
        });

        s.listen(target, move |s, _: &Updated, _, _| {
            // as above, we prevent an infinite cycle while undoing
            if !s.read(undoing, |v| *v).ok().flatten().unwrap_or_default() {
                // retrieve the information for our custom `Update` event
                let prev = s
                    .read(prev_values, |v| v.get(&target).copied())
                    .ok()
                    .flatten()
                    .flatten()
                    .unwrap_or_default();
                let next = s.read(target, |v| *v).ok().flatten().unwrap_or_default();
                // we no longer need the previous value
                _ = s.update(prev_values, |v| {
                    v.remove(&target);
                });

                // let's abandon our custom `Update` event if it is aborted, or if the votes to cancel >= the votes to proceed
                let undo = match s.emit(target, &Update { prev, next }) {
                    Ok(votes) => votes.cancel > votes.proceed,
                    Err(_) => true,
                };
                if undo {
                    // set the undo flag so we can revert our changes without entering an infinite cycle
                    _ = s.update(undoing, |v| *v = true);
                    // restore the target to its previous value
                    _ = s.update(target, |v| *v = prev);
                }
            } else {
                // the undoing is complete, we can reset the flag for the next time
                _ = s.update(undoing, |v| *v = false);
            }
        });

        s.listen(target, move |s, e: &Update, vote, _| {
            // while `should_cancel` is set, our `Update` event will always be cancelled, so no updates should happen
            let should_cancel = s.read(should_cancel, |v| *v).ok().flatten().unwrap_or_default();
            if should_cancel {
                *vote = Vote::Cancel;
                println!("-> prevented change: {} => {}", e.prev.0, e.next.0);
            } else {
                println!("-> made change: {} => {}", e.prev.0, e.next.0);
            }
        });
    });

    // our testing variable
    let x = system
        .create(|_, _| Item::default())
        .expect("no cancelling listeners registered");

    for line in std::io::stdin().lock().lines().flatten() {
        match line.as_str() {
            "@exit" => {
                println!("Exiting...");
                break;
            }
            "@toggle" => {
                // toggle whether or not subsequent `Update` events should be cancelled
                _ = system.update(should_cancel, |v| *v = !*v);
            }
            "@val" => {
                // print the current value of our testing variable `x`
                let x = system.read(x, |v| *v).ok().flatten().unwrap_or_default();
                println!("-> x = {}", x.0);
            }
            _ => {
                // otherwise we show of our `Update` event by setting our test variable to the # chars read
                _ = system.update(x, |v| *v = Item(line.chars().count()));
            }
        }
    }
}
