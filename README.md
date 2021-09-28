# Caw

`caw` is a library for data armoring using a shared list of indices into a dictionary.

**Please don't use this library for cryptography!**

This library is just a fancy substitution cipher, and security is not its main goal. If you want to produce really secure output, please use a real cipher to encrypt the data before encoding it with Caw.

## Implementing Caw

### Dictionary and DictionaryMapping

Caw uses the dictionary at `/words` for all of its actions. Changing this dictionary breaks compatibility, so you should retain it in your implementation. The dictionary was acquired from [this repository](https://github.com/dwyl/english-words), and its use is governed under [the Unlicense](https://unlicense.org/).

A pair of users communicating using Caw should share a 128-bit seed, and their computers should agree on the date. The seed and the date are used to seed `rand_pcg64`, and are used in `rand`'s `seq::SliceRandom::shuffle`. Implementors should ensure their Shuffle algorithm exactly matches `rand`'s. `shuffle` is used to generate a randomly ordered list of numbers from 0 to `words.len() - 1`, which is used to build a DictionaryMapping.

The first five entries in the list are assigned `begin`. The next five are assigned `end`, then the next five are assigned `fragment`. The following 65535 entries are assigned a 16-bit number equal to their position in the list, minus 16. Changing the number of aliases for `begin`, `end`, and `fragment` breaks compatibility, but in the future that length may be dependent on the seed to allow for larger alias counts.

### Encoding

1. Encoding requires the message as a list of bytes, the DictionaryMapping, the Dictionary, and a soft character limit.
   * The soft character limit may be exceeded slightly, so it's best to supply a smaller value than the real character limit.
3. Each pair of bytes in the message is joined into a 16-bit number, with the earlier byte becoming the high byte and the later byte becoming the low byte.
4. Each 16-bit number is mapped to its signifying word.
5. The `start` and `end` words are randomly chosen and added to the start and end of the message.
6. The exact implementation of splits isn't important for compatibility; implement them however you like, ensuring that every fragment other than `begin` starts with `fragment` followed by the zero-based position of the fragment in the order of the message. (So, the first non-`begin` fragment will have `1`, since the `begin` fragment was `0`.)
7. Success!

### Decoding

1. Decoding requires all fragments as a list of strings, the DictionaryMapping, and the Dictionary.
2. Map all of the words in all of the messages to their corresponding indices in the Dictionary, ignoring anything meaningless.
3. Sort messages by fragment order.
4. Strip `begin`, `fragment {N}`, and `end`.
5. Look up every word's 16-bit meaning in the DictionaryMapping, discarding anything meaningless.
6. Split each 16-bit number into two bytes, the high byte coming first.
7. Success!
