#![forbid(missing_docs)]

//! Caw is a library which can armor data using a simple time-sensitive substitution
//! cipher, intended to prevent automated identification of non-English data.

use std::time::Instant;

use chrono::{Date, Datelike, Utc};
use rand::prelude::SliceRandom;
use rand_pcg::Pcg64;
use rayon::prelude::*;

// rust-analyzer doesn't like this but it works
const DICTIONARY: &'static [&'static str] = &include!("../words");

/// The mappings between 16-bit words and English words.
#[derive(Debug)]
pub struct DictMappings {
	/// The corresponding indices for 16-bit words.
	pub words: Vec<u32>,
	/// The indices for the beginning of a message.
	pub begin: Vec<u32>,
	/// The indices for the end of a message.
	pub end: Vec<u32>,
	/// The indices for the start of a message fragment.
	pub fragment: Vec<u32>,
}

impl DictMappings {
	/// Build the dictionary mappings from a shared seed and the current date.
	pub fn from_seed(seed: u128, date: &Date<Utc>) -> DictMappings {
		use rand_seeder::Seeder;
		let mut rng: Pcg64 = Seeder::from(format!(
			"{}{}{}{}",
			seed,
			date.year(),
			date.month(),
			date.day()
		))
		.make_rng();
		let mut indices: Vec<u32> = (0..DICTIONARY.len() as u32).collect();
		indices.shuffle(&mut rng);
		DictMappings {
			begin: indices[0..5].to_vec(),
			end: indices[5..10].to_vec(),
			fragment: indices[10..15].to_vec(),
			words: indices[15..16 + (u16::MAX as usize)].to_vec(),
		}
	}
	/// Look up a 16-bit word given its index in the dictionary.
	pub fn reverse_lookup(&self, index: u32) -> Option<u16> {
		self.words
			.iter()
			.enumerate()
			.filter(|(_, v)| **v == index)
			.map(|(n, _)| n as u16)
			.next()
	}
}

/// Don armor. Returns a list of messages, including split headers.
pub fn don(data: &[u8], dict: &DictMappings, character_limit: usize) -> Vec<String> {
	let mut rng = rand::thread_rng();
	// Build the un-split list of words
	let mut words: Vec<&str> = data
		.par_chunks(2)
		// Convert each byte pair to a 16-bit word
		.map(|pair| {
			let a = pair[0];
			let b = *pair.get(1).unwrap_or(&0);
			((a as u16) << 8) + (b as u16)
		})
		// Map each 16-bit word into an index into the dictionary
		.map(|word| dict.words[word as usize])
		.map(|index| DICTIONARY[index as usize])
		.collect();
	// Write begin and end
	words.insert(
		0,
		DICTIONARY[*dict.begin.choose(&mut rng).unwrap() as usize],
	);
	words.push(DICTIONARY[*dict.end.choose(&mut rng).unwrap() as usize]);
	// The positions of each split.
	let mut splits: Vec<usize> = vec![0];
	let mut count = 0usize;
	let fragment_len = dict
		.fragment
		.iter()
		.map(|v| DICTIONARY[*v as usize].len())
		.max()
		.unwrap_or(0);
	for (index, word) in words.iter().enumerate() {
		if count != 0 {
			count += 1;
		}
		count += word.len();
		if count + fragment_len > character_limit {
			splits.push(index);
			count = word.len();
		}
	}
	splits.push(words.len());
	// Copy each range into its own message.
	splits
		.par_windows(2)
		.enumerate()
		.map(|(index, range)| {
			let start = range[0];
			let end = range[1];
			let mut rng = rand::thread_rng();
			let mut result: Vec<&str> = vec![];
			if index != 0 {
				result.push(DICTIONARY[*dict.fragment.choose(&mut rng).unwrap() as usize]);
				result.push(DICTIONARY[dict.words[index] as usize]);
			}
			result.append(
				&mut words
					.iter()
					.skip(start)
					.take(end - start)
					.copied()
					.collect(),
			);
			result.join(" ")
		})
		.collect()
}

/// Doff armor.
pub fn doff(messages: &[String], dict: &DictMappings) -> Vec<u8> {
	let indices: Vec<Vec<usize>> = messages
		.par_iter()
		.map(|v| {
			v.split(' ')
				.map(|v| {
					DICTIONARY
						.iter()
						.enumerate()
						.filter(|(_, w)| **w == v)
						.map(|(n, _)| n)
						.next()
				})
				.flatten()
				.collect()
		})
		.collect();
	let mut numbered_data: Vec<(u16, Vec<usize>)> = indices
		.par_iter()
		.map(|v| {
			if dict.begin.contains(&(v[0] as u32)) {
				(0, v[1..].to_vec())
			} else {
				assert!(dict.fragment.contains(&(v[0] as u32)));
				let index: u16 = dict.reverse_lookup(v[1] as u32).unwrap();
				(index, v[2..].to_vec())
			}
		})
		.collect();
	numbered_data.sort_by(|(a, _), (b, _)| a.cmp(b));
	let binary_data: Vec<u8> = numbered_data
		.par_iter()
		.flat_map(|(_, words)| {
			words
				.iter()
				.map(|v| dict.reverse_lookup(*v as u32))
				.flatten()
				.flat_map(|v| vec![(v >> 8) as u8, ((v << 8) >> 8) as u8])
				.collect::<Vec<u8>>()
		})
		.collect();
	binary_data
}

#[cfg(test)]
mod tests {
	use chrono::Utc;

	use crate::DictMappings;

	#[test]
	fn reversibility() {
		let dict = DictMappings::from_seed(69, &Utc::now().date());
		let test_data = "This is a very cool test string 😎".as_bytes();
		let resultant_data = super::don(test_data, &dict, 50);
		let doffed = super::doff(&resultant_data, &dict);
		assert_eq!(test_data, doffed);
	}
	#[test]
	fn reverse_lookup() {
		let dict = DictMappings::from_seed(69, &Utc::now().date());
		for word in vec![32551, 1233, 43241, 3289, 123, 1234, 1] {
			let entry = dict.words[word as usize];
			let reverse = dict.reverse_lookup(entry).unwrap();
			assert_eq!(word, reverse)
		}
	}
}
