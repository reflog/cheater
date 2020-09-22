extern crate glib;
extern crate gtk;

use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
mod helpers;

const CHEATSHEETS_DIR: include_dir::Dir = include_dir::include_dir!("./cheatsheets");

#[derive(Serialize, Deserialize, Clone)]
pub struct Cheatsheet {
    categories: Vec<Category>,
    title: String,
    classes: Option<Vec<String>>,
    window_names: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Category {
    name: String,
    commands: Vec<Command>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Command {
    command: String,
    name: String,
    notes: Option<String>,
}

fn i3_config() -> Option<Cheatsheet> {
    if !cfg!(feature = "i3") {
        return None;
    }
    use i3_ipc::Connect;

    let comment_regex =
        regex::Regex::new(r"##(?P<category>.*?)//(?P<title>.*?)//(?P<shortcut>.*?)##").unwrap();
    let mut sheet = Cheatsheet {
        categories: vec![],
        classes: None,
        window_names: None,
        title: "i3".to_string(),
    };
    if let Ok(mut i3) = i3_ipc::I3::connect() {
        let config = i3.get_config().unwrap_or(i3_ipc::reply::Config {
            config: "".to_string(),
        });
        let mut categories: HashMap<String, Category> = HashMap::new();
        for m in comment_regex.captures_iter(&config.config) {
            let cname = m["category"].trim().to_string();
            let name = cname.clone();
            let entry = categories.entry(cname).or_insert(Category {
                name,
                commands: vec![],
            });
            entry.commands.push(Command {
                command: helpers::clean_i3_shortcut(m["shortcut"].trim().to_string()),
                name: m["title"].trim().to_string(),
                notes: None,
            })
        }
        categories
            .values()
            .for_each(|e| sheet.categories.push(e.clone()));
    }
    Some(sheet)
}

fn scan_cheatsheets() -> Result<Vec<Cheatsheet>, String> {
    let mut sheets: Vec<Cheatsheet> = vec![];
    for entry in CHEATSHEETS_DIR.find("*.json").map_err(|e| e.to_string())? {
        let data_str = CHEATSHEETS_DIR
            .get_file(entry.path())
            .unwrap()
            .contents_utf8()
            .unwrap();
        let parsed: Cheatsheet = serde_json::from_str(&data_str).expect("cannot parse json");
        sheets.push(parsed);
    }
    if let Some(i3_sheet) = i3_config() {
        sheets.push(i3_sheet);
    }

    Ok(sheets)
}

fn sheet_from_window(sheets: &Vec<Cheatsheet>) -> Option<&Cheatsheet> {
    let mut sheet: Option<&Cheatsheet> = None;
    if let Ok((wm_name, wm_class)) = helpers::get_proc_and_focused_window_pid() {
        sheet = sheets.iter().find(|s| {
            if s.classes.clone().unwrap_or(vec![]).contains(&wm_class) {
                return true;
            }
            s.window_names
                .clone()
                .unwrap_or(vec![])
                .iter()
                .find(|x| x.contains(&wm_name))
                .is_some()
        });
    }
    if sheet.is_none() && cfg!(feature = "i3") {
        return sheets.iter().find(|x| x.title == "i3");
    }
    sheet
}

fn window_from_sheet(sheet: &Cheatsheet) {
    let window = gtk::ShortcutsWindowBuilder::new().modal(true).build();
    window.set_property_window_position(gtk::WindowPosition::CenterAlways);
    let section = gtk::ShortcutsSectionBuilder::new().build();
    for group_data in sheet.clone().categories {
        let group = gtk::ShortcutsGroupBuilder::new().build();
        group.set_property_title(Some(&group_data.name));
        for shortcut_data in group_data.commands {
            let shortcut = gtk::ShortcutsShortcutBuilder::new().build();
            shortcut.set_property_title(Some(&shortcut_data.name));
            shortcut.set_property_accelerator(Some(&shortcut_data.command));
            group.add(&shortcut);
        }
        section.add(&group);
    }
    section.show();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(true)
    });
    window.add(&section);
    window.show_all();
    window.show();
}

pub fn main() {
    let sheets: Vec<Cheatsheet> = scan_cheatsheets().expect("faild to scan cheatsheets");

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    if let Some(sheet) = sheet_from_window(&sheets) {
        window_from_sheet(sheet);
    }

    gtk::main();
}
