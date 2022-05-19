#![no_main]
#![no_std]

use alloc::{string::String, vec::Vec};
use standard::{print, println, syscalls::get_input};
#[macro_use]
extern crate alloc;

fn stringify(guessed_word: &Vec<char>) -> String {
	let mut s = String::new();
	for c in guessed_word {
		s.push(*c);
		s.push(' ');
	}
	s
}

#[no_mangle]
pub extern "C" fn main() -> isize {
	let word = "hello";

	let mut guessed_word = vec!['_'; word.len()];
	let mut state = 0;
	let mut guesses: Vec<char> = Vec::new();
	loop {
		println!("\n{}", stringify(&guessed_word));
		print!("Guess a letter: ");
		let mut buf = [0; 1];
		get_input(&mut buf);
		let guess = buf[0] as char;
		print!("{}", guess);
		if !guess.is_alphabetic() {
			// print!("\nThat's, like, literally not a fucking letter like ???");
			continue;
		}

		if guesses.contains(&guess) {
			println!("\nLetter already guessed!")
		} else {
			if word.contains(guess) {
				for (i, letter) in word.chars().enumerate() {
					if letter == guess {
						guessed_word[i] = guess;
					}
				}
			} else {
				guesses.push(guess);
				state += 1;
			}
			if state == HANGMANS.len() {
				println!("\nYou lose! The word was {}.", word);
				break;
			}
			println!("\n{}", HANGMANS[state]);
			println!("{:?}", guesses);
			if !guessed_word.contains(&'_') {
				println!("You win!");
				break;
			}
		}
	}
	return 0;
}

const HANGMANS: [&'static str; 7] = [
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
