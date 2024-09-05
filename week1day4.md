# Thinking about it, reviewinging
So essentially, this day the SST (sorted string table) contains how strings are stored on disk.
Each block (4k blocks) data is stored on disk followed by the index (SST part), followed by how many entries are. The SST tells the LSM what block to load for seeking to key. Each index contains the beginning and end keys (assumed to be sorted on disk) of what is in this block. The iterator goes and compares the first and the current key and if the key searched is greater than the block key in the SST, it loads the block. The entire SST is loaded, containing the offset of block (u64), key_start and key_end (maybe 256 max?, there is no limit haha since it's all loaded). The iterator also uses the block iterator to find the next key. The cache is just mapping from (sst_it, block_idx) to Block that's loaded. How it evicts? probably lru, yes (https://docs.rs/moka/latest/moka/)

blocks should be around 4k, they contain [[key_len][value_len][key][value]] ... as a buffer and [offset] ... in memory
then on disk, it's [[key_len][value_len][key][value]] ... [offset] ... [num offsets]

Then in sst, the index in memory is
[[offset][key_first][key_last]] and it's appended to the blocks
[[[key_len][value_len][key][value]] ... [offset] ... [num offsets]] ... [[offset][key_first][key_last]] ... [num index entries]

week1day1
How is the memtable laid out/immutable memtable?
the memtable is a skiplist. it's sorted and search is log(n). When the memtable reaches a certain size (estimated by key and value sizes), then it becomes immutable. It gets added to the beginning since it's the latest one up to date [up to date][...][older...]

How does the memtable iterate?
You start at the first entry, and compare to the next entry in memtable, if not, just walk the memtable until the end. Then you start to compare against the immutable tables.

How does the block iterate?
The block iterates by each entry until the key is found. It's O(n).

How does the sst iterate?
The SST searches through the index, (key_first) until key is >= key_first. Then it loads the block and then iterates through each entry in the block until the key is found!

# Questions
What is the time complexity of seeking a key in the SST?
- O(n) entry for each entry in the block. For each entry of the index in the sst, you have to iterate through the index to find the correcct block.

Where does the cursor stop when you seek a non-existent key in your implementation?
- It starts at the beginnning of the SST/first block again.

Is it possible (or necessary) to do in-place updates of SST files?
- No, the index and the block needs to be updated separately. It needs to be written in one go, son you need a lock.

An SST is usually large (i.e., 256MB). In this case, the cost of copying/expanding the Vec would be significant. Does your implementation allocate enough space for your SST builder in advance? How did you implement it?
- I don't expand it all to 256MB. You can though, and ideally should. But it's a tradeoff since you need to keep it in memory and holding that much memory. I build it at the end (cpu vs memory tradeoff)

Looking at the moka block cache, why does it return Arc<Error> instead of the original Error?
- It's threadsafe, it can have reference to the same error

Does the usage of a block cache guarantee that there will be at most a fixed number of blocks in memory? For example, if you have a moka block cache of 4GB and block size of 4KB, will there be more than 4GB/4KB number of blocks in memory at the same time?
-  No, it gives best amount. It can spill. (best-effort bounding of the map)

Is it possible to store columnar data (i.e., a table of 100 integer columns) in an LSM engine? Is the current SST format still a good choice?
- No, I think you need to separate each into one block of the same item and toy need to index them in the same way. Structure of arrays vs array of structures, currently SST is array of structures, not key value

Consider the case that the LSM engine is built on object store services (i.e., S3). How would you optimize/change the SST format/parameters and the block cache to make it suitable for such services?
- Depends on variable SST size and compress.

For now, we load the index of all SSTs into the memory. Assume you have a 16GB memory reserved for the indexes, can you estimate the maximum size of the database your LSM system can support? (That's why you need an index cache!)
- 256MB sst, 4k per data, u64 (8) + first_key(64) + last_key(64) = ~128
- 256MB / 4k + 128 = 256000000 / (4096 + 128) = 60k entries per 256MB
- 16MB / 4k = 4000 * 256MB = 8Tb?, but it's ~ 2TB? at least i'm in same range
- but for number of total entries 240million entries
