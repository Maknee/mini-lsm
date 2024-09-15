# Working through

Goals:
- Implement bloom filter on SSTs and integrate into the LSM read path get.
- Implement key compression in SST block format.

Working through:
Task 1: Bloom Filters

Finally I get to work on bloom filters. I've never used them in practice. Only hear about them. 

[A link to understand them](https://samwho.dev/bloom-filters/)

Constructor sets the key bits of the keys and then the contains just uses `let delta = (h >> 17) | (h << 15);` for each `k` hashes you want to check. Compress? total 32 bits for k bits

Task 2: Integrate Bloom Filter on the Read Path

Basically build a bloom filter into SsTableBuilder. It keeps track of hashes of all keys added. When the SsTableBuilder builds the SsTable, it builds the entire bloom filter and flushes appends it to the disk. Now the disk format is

[[data block]..][[index block]..][[bloom filter]][[meta len][meta offset][bloom len][bloom off]]

Task 3: Key Prefix Encoding + Decoding

src/block/builder.rs
src/block/iterator.rs

Actually got the thing to work. Damn my SstableBuilder wasn't actually working correctly ~ I was building a metadata entry for each entry that was added. Basically for each add, I finish that block haha. (true vs false and using enums would be MUCH better)

So instead, what needs to be done is to add block until the block is basically full. Now, you need to finish the table (build it and add to meta). A small thing to note is that a) you have to add the current entry that wasn't added into a new block. And b) you need to finish the block at the end with you call build for the entire sstable.

In addition, when you find a block, you need have a start and end of the block. If the blocks are (first key, last key) [0, 15] and [20, 35], and you request for key 16, you need to give back the next block (makes sense for iteration). So you need to check if a) the current key is greater than the first key and less than the last key. (0 < 16 < 15) If not, you have to check if the current key is less than the current key, and if so, you get back 20 instead because 16 < 20.

The prefix decoding is basically sticking the first key into data block and having X bytes of it be part of the other entries (saving some bytes hopefully!). So basically if you have `mini-something`, and next key is `mini-LSM`, you have 5 characters overlapping `mini-` and then the remaining is `LSM`. Does this save bytes? so instead of 8, you have 5|3|LSM (u16) + (u16) + 3 (LSM) = 7 instead of 8. Cool.

# Thinking about it, reviewing

Task 1: Bloom Filters

Bloom filters basically check if item is not in set quickly by checking random bits.

Task 2: Integrate Bloom Filter on the Read Path

We add it to gets for sstable to quickly check if the key is even in the sstable.

Task 3: Key Prefix Encoding + Decoding

We compress the sstable a bit (not sure how much)

Overall:

Nice little additions and I found a bug!

# Questions
How does the bloom filter help with the SST filtering process? What kind of information can it tell you about a key? (may not exist/may exist/must exist/must not exist)

The bloom filter basically checks if the block has even your key, so you don't have to scan through the block if needed once you load it.

Consider the case that we need a backward iterator. Does our key compression affect backward iterators?

Yes. You can still have a backwareds iterator with key compression. You just need to load the first key.

Can you use bloom filters on scan?

Yes. You can check if some keys exist or not and just filter keys you need to search for the scan.

What might be the pros/cons of doing key-prefix encoding over adjacent keys instead of with the first key in the block?

Saves space. You can encode more over adjacent keys instead of just the first key. The latter keys might have less overlap with the first key. This increases computation (by a little bit), worth the space? Maybe.

# Answers according to claude

Prompt:
Are the answers to the questions correct? If it is, state that it is and if the answer doesn't answer the question well, expand. If it is wrong, explain that it is wrong and then explain what is correct

I'll review each question and answer, confirming correct responses and expanding or correcting as necessary.

Question: How does the bloom filter help with the SST filtering process? What kind of information can it tell you about a key? (may not exist/may exist/must exist/must not exist)
Answer: The bloom filter basically checks if the block has even your key, so you don't have to scan through the block if needed once you load it.

This answer is partially correct but doesn't fully address the question. Let's expand:

The bloom filter indeed helps check if a key might be in a block without having to load and scan the entire block. This improves efficiency by potentially avoiding unnecessary I/O operations. However, the answer doesn't address the second part of the question about the kinds of information a bloom filter can provide.

A bloom filter can tell you two things about a key:

May exist: If the bloom filter indicates a positive result, the key may exist in the block. However, this can be a false positive.
Must not exist: If the bloom filter indicates a negative result, the key definitely does not exist in the block. There are no false negatives.
A bloom filter cannot tell you with certainty that a key "must exist" or provide information about keys that "may not exist" (as this is equivalent to "may exist").

Question: Consider the case that we need a backward iterator. Does our key compression affect backward iterators?
Answer: Yes. You can still have a backwards iterator with key compression. You just need to load the first key.

This answer is correct but could be expanded for clarity:

Key compression does affect backward iterators, but not in a way that prevents their use. With key compression, typically only the first key in a block is stored in full, with subsequent keys stored as deltas from the previous key. For a backward iterator to work, you need to load the first (uncompressed) key of the block and then decompress the keys in reverse order. This allows you to reconstruct the full keys as you iterate backwards through the block.

Question: Can you use bloom filters on scan?
Answer: Yes. You can check if some keys exist or not and just filter keys you need to search for the scan.

This answer is correct and provides a good basic explanation. To expand slightly:

Bloom filters can be used to optimize scan operations in certain scenarios. When performing a range scan, you can use the bloom filter to quickly check if certain keys or key prefixes might exist in a block. This can help you skip entire blocks that don't contain relevant keys for your scan range, potentially reducing the number of blocks you need to read and process.

Question: What might be the pros/cons of doing key-prefix encoding over adjacent keys instead of with the first key in the block?
Answer: Saves space. You can encode more over adjacent keys instead of just the first key. The latter keys might have less overlap with the first key. This increases computation (by a little bit), worth the space? Maybe.

This answer is correct and provides a good overview of the main trade-offs. It accurately identifies the primary advantage (space savings) and the main disadvantage (increased computation). The answer also correctly notes that the space-time trade-off may or may not be worthwhile, depending on the specific use case and requirements.

