# Thinking about it, reviewing

Task 1: Flush Memtable to SST

Force a flush to memtable on put, sync check if immtables hit a limit `LSMStorageInner::force_flush_next_imm_memtable`. This locks the state, pulls the last immemtable and then builds an SST using SSTTableBuilder. Then inserts this as a new entry to the sstables. Then we remove the immemtable.

Do note that the immemtables are stored as last one as earliest, which makes sense.

Task 2: Flush Trigger

Then we modify a crossbeam thread that does flushing in the background every 50ms, basically just checking the length of immemtable.

Task 3: Filter the SSTs

Now, it works! We can use the CLI to write/read/scan the database.

Overall:

Pretty fucking cool that it finally works.

# Questions

What happens if a user requests to delete a key twice?

The immemtables becomes an SST and the earliest should state that it's deleted.

How much memory (or number of blocks) will be loaded into memory at the same time when the iterator is initialized?

The scan iterator? The immemtables + memtable + one block (4k/8k/16k bytes) at a time.

Some crazy users want to fork their LSM tree. They want to start the engine to ingest some data, and then fork it, so that they get two identical dataset and then operate on them separately. An easy but not efficient way to implement is to simply copy all SSTs and the in-memory structures to a new directory and start the engine. However, note that we never modify the on-disk files, and we can actually reuse the SST files from the parent engine. How do you think you can implement this fork functionality efficiently without copying data? (Check out Neon Branching).

Some type of copy of write mechanism. So if the in memory structure is modified, fork it to your own LSM engine. The SST files will remain the same (they do not change, but maybe during compaction they will, so you may need to check if they're being referenced)

Imagine you are building a multi-tenant LSM system where you host 10k databases on a single 128GB memory machine. The memtable size limit is set to 256MB. How much memory for memtable do you need for this setup?

128GB / 10k = 1.2GB per database, so like 4 or 5 memtables.

Obviously, you don't have enough memory for all these memtables. Assume each user still has their own memtable, how can you design the memtable flush policy to make it work? Does it make sense to make all these users share the same memtable (i.e., by encoding a tenant ID as the key prefix)?

Yeah, skiplists are concurrent, so there's not any locks besides freezing the memtable. It also depends on how many writes there will be.

# Answers according to chatgpt3.5

Prompt:
Are the answers to the questions correct? If it is, state that it is and if the answer doesn't answer the question well, expand. If it is wrong, explain that it is wrong and then explain what is correct

1. What happens if a user requests to delete a key twice?
Answer:

"The immutables become an SST and the earliest should state that it's deleted."
Evaluation:

This is mostly correct but lacks some clarity. When a key is deleted, it is marked with a tombstone in the memtable (or an immutable memtable). When the memtable is flushed to disk, the deletion record is written to the SST. If the user tries to delete the key again, the deletion operation is processed in a similar way, but it won't affect the already deleted key because LSM trees use timestamps to keep track of the latest version. The earliest version will indeed reflect that the key has been deleted, and this tombstone record will persist until compaction removes it, once it is no longer needed.
Conclusion: The answer is correct, but it could be clearer about the role of tombstones in LSM trees.

2. How much memory (or number of blocks) will be loaded into memory at the same time when the iterator is initialized?
Answer:

"The scan iterator? The immutables + memtable + one block (4k/8k/16k bytes) at a time."
Evaluation:

This is correct. When the iterator is initialized, it needs to scan through the memtable, immutable memtables, and the SST files. SST files are accessed block by block, typically in sizes like 4KB, 8KB, or 16KB, depending on the block size. Only one block is loaded at a time from disk, while the memtable and immutables are in-memory.
Conclusion: The answer is correct.

3. How to implement an efficient fork functionality without copying data (similar to Neon Branching)?
Answer:

"Some type of copy-on-write mechanism. If the in-memory structure is modified, fork it to your own LSM engine. The SST files will remain the same (they do not change, but maybe during compaction they will, so you may need to check if they're being referenced)."
Evaluation:

This is correct. Forking an LSM tree can be done using a copy-on-write mechanism. Since SST files are immutable, they can be shared between the parent and child engines. The forked engine would only need to copy the in-memory structures (like the memtable). However, when compaction occurs, you may need to be careful if multiple engines reference the same SST files. The reference counting or tracking mechanism ensures the SST files are only cleaned up once they are no longer needed by any engine.
Conclusion: The answer is correct.

4. How much memory for memtables is needed when hosting 10k databases with a 256MB memtable size limit on a 128GB machine?
Answer:

"128GB / 10k = 1.2GB per database, so like 4 or 5 memtables."
Evaluation:

This is incorrect. The calculation seems off. If the machine has 128GB of memory and there are 10k databases, you would get approximately 12.8MB per database for memtable usage (128GB ÷ 10,000 = 12.8MB per database). This is far less than the 256MB memtable size limit, so clearly, not all databases can have an active 256MB memtable at the same time.
Conclusion: The memory per database is approximately 12.8MB, so you cannot fit 4 or 5 memtables per database, but only a fraction of one memtable per database.

5. How to design a memtable flush policy for a multi-tenant LSM system? Does it make sense to share a memtable by encoding a tenant ID as a key prefix?
Answer:

"Yeah, skiplists are concurrent, so there's not any locks besides freezing the memtable. It also depends on how many writes there will be."
Evaluation:

This answer touches on some aspects but doesn't fully address the question. If each user has their own memtable and you cannot fit all memtables into memory, a possible approach is to flush memtables to disk more aggressively, especially for inactive tenants. A shared memtable across tenants by encoding a tenant ID as a key prefix could work, but it might introduce contention in high-write scenarios. You would need to balance concurrency and lock contention. Alternatively, a multi-tiered flushing mechanism could be used where hot tenants keep larger in-memory structures while cold tenants are flushed to disk more frequently.
Conclusion: The answer is partially correct but doesn't discuss the potential issues with sharing memtables (e.g., contention and complexity) or propose detailed solutions like tenant-specific flush strategies.
