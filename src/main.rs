use fastnbt::Value;
use flate2::read::GzDecoder;
use flate2::Compression;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use flate2::write::GzEncoder;

fn main() {
    let mut args = env::args();
    let Some(executable) = args.next() else { print_usage_and_exit(None) };
    let Some(scoreboard_file) = args.next() else { print_usage_and_exit(Some(executable)) };
    let Some(player_from) = args.next() else { print_usage_and_exit(Some(executable)) };
    let Some(player_into) = args.next() else { print_usage_and_exit(Some(executable)) };

    let mut scoreboard = match read_scoreboard_file(&scoreboard_file) {
        Ok(scoreboard) => scoreboard,
        Err(err) => {
            println!("Unable to read scoreboard file: {}", err);
            return;
        }
    };
    let Value::Compound(scoreboard_data) = &mut scoreboard else {
        println!("Scoreboard not a compound");
        return;
    };
    let Some(Value::Compound(scoreboard_data)) = scoreboard_data.get_mut("data") else {
        println!("Scoreboard data not a compound");
        return;
    };
    let Some(Value::List(player_scores)) = scoreboard_data.get_mut("PlayerScores") else {
        println!("Could not find player scores");
        return;
    };

    let mut old_scores = HashMap::new();

    let mut i = 0;
    while i < player_scores.len() {
        if let Value::Compound(compound) = &player_scores[i] {
            if let Some(Value::String(name)) = compound.get("Name") {
                if name == &player_from {
                    if let (Some(Value::String(objective)), Some(Value::Int(score))) =
                        (&compound.get("Objective"), compound.get("Score"))
                    {
                        old_scores.insert(objective.clone(), *score);
                        player_scores.remove(i);
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    println!("Could not find player {} in scoreboard", player_from);

    let mut found_into = false;
    for new_score in &mut *player_scores {
        let Value::Compound(new_score) = new_score else { continue };
        let Some(Value::String(name)) = new_score.get("Name") else { continue };
        if name != &player_into {
            continue;
        }
        found_into = true;
        let Some(Value::String(objective)) = new_score.get("Objective") else { continue };
        let Some(old_score) = old_scores.remove(objective) else { continue };
        let Some(Value::Int(new_score)) = new_score.get_mut("Score") else { continue };
        *new_score = new_score.wrapping_add(old_score);
    }

    if !found_into {
        println!("Could not find player {} in scoreboard", &player_into);
    }

    for (objective, score) in old_scores {
        let mut new_entry = HashMap::with_capacity(4);
        new_entry.insert("Name".to_owned(), Value::String(player_into.clone()));
        new_entry.insert("Objective".to_owned(), Value::String(objective));
        new_entry.insert("Score".to_owned(), Value::Int(score));
        new_entry.insert("Locked".to_owned(), Value::Byte(0));
        player_scores.push(Value::Compound(new_entry));
    }

    if let Err(err) = write_scoreboard_file(&scoreboard_file, &scoreboard) {
        println!("Unable to write scoreboard file: {}", err);
        return;
    }

    println!("Done.");
}

fn print_usage_and_exit(executable: Option<String>) -> ! {
    println!(
        "{} <scoreboard_file> <player_from> <player_into>",
        executable.as_deref().unwrap_or("scoreboard_merger")
    );
    std::process::exit(0);
}

fn read_scoreboard_file(path: &str) -> Result<Value, Box<dyn Error>> {
    let file = File::open(path)?;
    Ok(fastnbt::from_reader(GzDecoder::new(BufReader::new(file)))?)
}

fn write_scoreboard_file(path: &str, scoreboard: &Value) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    fastnbt::to_writer(GzEncoder::new(BufWriter::new(file), Compression::default()), scoreboard)?;
    Ok(())
}
