#![no_main]
#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use standard::{io::*, syscalls::*, *};

use core::{fmt, ops};

use serde::{Deserialize, Serialize};

#[no_mangle]
pub extern "C" fn main() -> isize {
	let board = Board::default();
	board.solve();
	return 0;
}

const SIZE: usize = 6;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Tile {
	Black,
	White,
	Unknown,
}

impl fmt::Debug for Tile {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let tile = match self {
			Tile::Black => '#',
			Tile::White => ' ',
			Tile::Unknown => '?',
		};
		write!(f, "{}", tile)
	}
}

impl ops::Add<Tile> for &Tile {
	type Output = Tile;
	fn add(self, rhs: Tile) -> Tile {
		match self {
			Tile::Unknown => Tile::Unknown,
			_ => {
				if *self == rhs {
					*self
				} else {
					Tile::Unknown
				}
			}
		}
	}
}

type Line = [Tile; SIZE];

type Header = Vec<usize>;

type SideHeader = [Header; SIZE];

type Options = Vec<Line>;

#[derive(Debug, Serialize, Deserialize)]
struct Board {
	row: SideHeader,
	column: SideHeader,
}

impl Board {
	fn solve(&self) {
		let mut row_options: Vec<Options> = self.row.iter().map(find_options).collect();
		let mut col_options: Vec<Options> = self.column.iter().map(find_options).collect();
		let mut row_summaries: Vec<Line> = row_options.iter().map(|x| summarize(&x)).collect();
		let mut col_summaries: Vec<Line> = col_options.iter().map(|x| summarize(&x)).collect();
		while !done(&row_summaries) {
			filter(&row_summaries, &mut col_options);
			filter(&col_summaries, &mut row_options);
			row_summaries = row_options.iter().map(|x| summarize(&x)).collect();
			col_summaries = col_options.iter().map(|x| summarize(&x)).collect();
		}

		for row in row_summaries.iter() {
			for square in row {
				print!("{:?} ", square);
			}
			println!();
		}
		println!();
	}
}

fn filter(from: &Vec<Line>, to: &mut Vec<Options>) {
	for (l, line) in from.iter().enumerate() {
		for (t, tile) in line.iter().enumerate() {
			match tile {
				Tile::Unknown => (),
				_ => {
					to[t].retain(|option_col| option_col[l] == *tile);
				}
			};
		}
	}
}

fn done(lines: &Vec<Line>) -> bool {
	for line in lines.iter() {
		for square in line.iter() {
			match square {
				Tile::Unknown => return false,
				_ => (),
			}
		}
	}
	true
}

fn min_size(header: &[usize]) -> usize {
	if header.len() > 0 {
		let mut sum = 0;
		for val in header.iter() {
			sum += val;
		}
		sum + header.len()
	} else {
		0
	}
}

fn find_options(header: &Header) -> Options {
	let mut res = Vec::<Line>::new();
	rec_find_options(&header[..], [Tile::White; SIZE], 0, &mut res);
	res
}

fn rec_find_options(header: &[usize], line: Line, filled: usize, result: &mut Options) {
	if header.len() > 0 {
		let remaining = &header[1..];
		for i in filled..SIZE - min_size(remaining) - header[0] + 1 {
			let mut copy = line;
			for j in i..i + header[0] {
				copy[j] = Tile::Black;
			}
			rec_find_options(&remaining, copy, i + header[0] + 1, result);
		}
	} else {
		result.push(line);
	}
}

fn summarize(options: &Options) -> Line {
	let mut res = options[0];
	for line in options.iter() {
		for (i, tile) in line.iter().enumerate() {
			res[i] = tile + res[i];
		}
	}
	res
}

impl Default for Board {
	fn default() -> Self {
		let mut file = File::open("/puzzle6by6.json").unwrap();
		let mut buf = Vec::new();

		file.read_to_end(&mut buf).expect("Failed to read!");

		let data = String::from_utf8(buf).unwrap();

		serde_json::from_str(&data).expect("bad json")

		// Board {
		// 	row: [vec![1]],
		// 	column: [vec![1]],
		// }

		// Board {
		// 	row: [
		// 		vec![6],
		// 		vec![14],
		// 		vec![18],
		// 		vec![22],
		// 		vec![16, 10],
		// 		vec![17, 11],
		// 		vec![32],
		// 		vec![32],
		// 		vec![33],
		// 		vec![34],
		// 		vec![35],
		// 		vec![15, 9],
		// 		vec![12, 7],
		// 		vec![11, 5],
		// 		vec![10, 5],
		// 		vec![9, 4],
		// 		vec![10, 3, 4],
		// 		vec![20, 3],
		// 		vec![21, 9, 4],
		// 		vec![21, 6, 1, 4],
		// 		vec![16, 3, 1, 4, 5],
		// 		vec![15, 2, 2, 6],
		// 		vec![13, 2, 8],
		// 		vec![10, 10],
		// 		vec![11, 9],
		// 		vec![14, 8],
		// 		vec![14, 2, 9],
		// 		vec![15, 7, 8],
		// 		vec![16, 3, 8],
		// 		vec![16, 2, 8],
		// 		vec![15, 3, 9],
		// 		vec![15, 4, 9],
		// 		vec![25, 9],
		// 		vec![28, 9],
		// 		vec![17, 3, 8, 4],
		// 		vec![23, 8],
		// 		vec![25, 8, 1],
		// 		vec![18, 10, 2],
		// 		vec![16, 12],
		// 		vec![17, 1, 11],
		// 		vec![19, 16],
		// 		vec![41],
		// 		vec![38],
		// 		vec![39],
		// 		vec![1, 32, 1],
		// 		vec![30],
		// 		vec![30],
		// 		vec![30, 3],
		// 		vec![30, 7],
		// 		vec![18, 9],
		// 	],
		// 	column: [
		// 		vec![],
		// 		vec![1],
		// 		vec![9],
		// 		vec![13],
		// 		vec![4, 1, 15, 2],
		// 		vec![32],
		// 		vec![34],
		// 		vec![34],
		// 		vec![36],
		// 		vec![39],
		// 		vec![44],
		// 		vec![45],
		// 		vec![46],
		// 		vec![46],
		// 		vec![11, 6, 25],
		// 		vec![11, 6, 24],
		// 		vec![10, 7, 23],
		// 		vec![9, 7, 2, 18],
		// 		vec![10, 6, 18],
		// 		vec![10, 4, 18],
		// 		vec![11, 4, 3, 15],
		// 		vec![10, 3, 7, 10],
		// 		vec![10, 4, 2, 7, 10],
		// 		vec![10, 6, 11, 9],
		// 		vec![11, 5, 5, 2, 2, 9],
		// 		vec![11, 1, 2, 2, 9],
		// 		vec![11, 1, 2, 1, 9],
		// 		vec![11, 1, 1, 1, 9],
		// 		vec![4, 5, 2, 1, 1, 8],
		// 		vec![11, 3, 1, 8],
		// 		vec![10, 2, 9],
		// 		vec![10, 3, 10],
		// 		vec![10, 3, 11],
		// 		vec![11, 3, 10],
		// 		vec![10, 1, 2, 13],
		// 		vec![11, 2, 1, 15],
		// 		vec![10, 1, 16],
		// 		vec![11, 20],
		// 		vec![46],
		// 		vec![44],
		// 		vec![39],
		// 		vec![8, 26],
		// 		vec![25, 2],
		// 		vec![14, 5, 1, 2],
		// 		vec![12, 4, 2],
		// 		vec![13, 2, 2, 1],
		// 		vec![2, 1, 1, 2, 1, 1],
		// 		vec![1, 1, 1],
		// 		vec![1, 1],
		// 		vec![],
		// 	],
		// }
	}
}
