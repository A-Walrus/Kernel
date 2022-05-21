#![no_main]
#![no_std]

use alloc::{string::String, vec::Vec};
use standard::{print, println, syscalls::get_input};
#[macro_use]
extern crate alloc;

fn stringify(guessed_word: &Vec<char>) -> String {
	let mut s = String::new();
	for c in guessed_word {
		if *c == '_' || *c == ' ' {
			s.push(*c);
		} else {
			s.push_str("\x1b[32m\x1b[4m");
			s.push(*c);
			s.push_str("\x1b[0m");
		}
		s.push(' ');
	}
	s
}
fn stringify_guesses(guesses: &Vec<char>) -> String {
	let mut s = String::new();
	s.push_str("[");
	for c in guesses {
		s.push_str("\x1b[31m");
		s.push(*c);
		s.push_str("\x1b[0m");
		if Some(c) != guesses.last() {
			s.push_str(", ");
		}
	}
	s.push_str("]");
	s
}

#[no_mangle]
pub extern "C" fn main() -> isize {
	let original_word = "Rust";
	let lower_word = original_word.to_lowercase();

	let mut guessed_word = vec!['_'; original_word.len()];
	for (i, letter) in original_word.chars().enumerate() {
		if letter == ' ' {
			guessed_word[i] = ' ';
		}
	}
	let mut state = 0;
	let mut guesses: Vec<char> = Vec::new();
	loop {
		println!("\n{}", stringify(&guessed_word));
		if !guessed_word.contains(&'_') {
			println!("You win!");
			break;
		}

		print!("Guess a letter: ");
		let mut buf = [0; 1];
		get_input(&mut buf);
		let guess = buf[0] as char;
		print!("{}", guess);
		if !guess.is_alphabetic() {
			// print!("\nThat's, like, literally not a fucking letter like ???");
			continue;
		}

		let guess = guess.to_ascii_lowercase();
		if guesses.contains(&guess) || guessed_word.contains(&guess) {
			println!("\n\x1b[33mLetter already guessed!\x1b[0m")
		} else {
			if lower_word.contains(guess) {
				for (i, letter) in lower_word.chars().enumerate() {
					if letter == guess {
						guessed_word[i] = original_word.chars().nth(i).unwrap();
					}
				}
			} else {
				guesses.push(guess);
				state += 1;
			}

			println!("\n{}", HANGMANS[state]);
			println!("{}", stringify_guesses(&guesses));
			if state == HANGMANS.len() - 1 {
				println!("\n\x1b[31mYou lose!\x1b[0m The word was \"{}\".", original_word);
				break;
			}
		}
	}
	return 0;
}

const HANGMANS: [&'static str; 8] = [
	r#"
  +---+
      |
      |
      |
      |
      |
========="#,
	r#"
  +---+
  |   |
      |
      |
      |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
      |
      |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
  |   |
      |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
 /|   |
      |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
 /|\  |
      |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
 /|\  |
 /    |
      |
========="#,
	r#"
  +---+
  |   |
  O   |
 /|\  |
 / \  |
      |
========="#,
];
