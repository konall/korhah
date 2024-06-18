use korhah::{events::Created, System, Variable};

use std::io::BufRead;

#[derive(Debug)]
struct InputEvent(String);

#[derive(Debug, Clone, Default)]
struct State {
    focused: Option<Variable<Element>>,
}

#[derive(Debug, Clone, Default)]
struct Element {
    parent: Option<Variable<Element>>,
    children: Vec<Variable<Element>>,
    text: Option<String>,
}

fn main() {
    let mut dom = System::default();

    // the state is
    let state = dom
        .create(|_, _| State::default())
        .expect("no cancelling listeners registered");

    // automatically establish parent-child relationships when a new element is added to the DOM
    dom.listen(None, move |dom, e: &Created<Element>, _, _| {
        // should only be done if the new element has a parent element of course
        if let Ok(Some(Some(parent))) = dom.read(e.source, |el| el.parent) {
            _ = dom.update(parent, |el| el.children.push(e.source));
        }
    });

    // the `body` element will be the parent of the other elements
    let body = dom
        .create(|_, _| Element::default())
        .expect("no cancelling listeners registered");

    // the `input` element will receive `InputEvent`s and update its text to match
    let input = dom
        .create(move |_, _| Element {
            parent: Some(body),
            text: Some("".into()),
            children: vec![],
        })
        .expect("no cancelling listeners registered");

    // receive `InputEvent`s on the `input` element
    dom.listen(input, move |dom, e: &InputEvent, _, _| {
        _ = dom.update(input, |el| {
            if let Some(text) = el.text.as_mut() {
                text.push_str(&e.0);
            } else {
                el.text = Some(e.0.to_owned());
            }
        });
    });

    // the `p` element will automatically update its text to match the text "entered" in the `input` element
    let p = dom
        .create(move |dom, _| Element {
            parent: Some(body),
            text: dom
                .read(input, |el| el.text.clone())
                .ok()
                .flatten()
                .expect("no variables deleted yet"),
            children: vec![],
        })
        .expect("no cancelling listeners registered");

    for line in std::io::stdin().lock().lines().flatten() {
        match line.as_str() {
            "exit" => {
                println!("Exiting...");
                break;
            }
            // focus different elements
            "@" => {
                println!("-> removing focus");
                _ = dom.update(state, |state| state.focused = None);
            }
            "@body" => {
                println!("-> focus `body`");
                _ = dom.update(state, |state| state.focused = Some(body));
            }
            "@input" => {
                println!("-> focus `input`");
                _ = dom.update(state, |state| state.focused = Some(input));
            }
            "@p" => {
                println!("-> focus `p`");
                _ = dom.update(state, |state| state.focused = Some(p));
            }
            // retrieve info
            "#state" => {
                _ = dom.read(state, |state| println!("STATE: {state:?}"));
            }
            "#dom" => {
                _ = dom.read(body, |el| println!("BODY: {el:?}"));
                _ = dom.read(input, |el| println!("INPUT: {el:?}"));
                _ = dom.read(p, |el| println!("P: {el:?}"));
            }
            "#body" => {
                _ = dom.read(body, |el| println!("BODY: {el:?}"));
            }
            "#input" => {
                _ = dom.read(input, |el| println!("INPUT: {el:?}"));
            }
            "#p" => {
                _ = dom.read(p, |el| println!("P: {el:?}"));
            }
            // clear the `input` element
            "$clear" => {
                println!("-> clearing `input`");
                _ = dom.update(input, |el| el.text = None);
            }
            _ => {
                if let Ok(Some(Some(focused))) = dom.read(state, |state| state.focused) {
                    let event = InputEvent(line);
                    println!("-> emitting {event:?}");
                    _ = dom.emit(focused, &event);
                } else {
                    println!("-> no element has focus");
                }
            }
        }
    }
}
