# Thinking about it, reviewing

This day, I worked on get function to read memtable and sst after. Basically linking the read between memtable and sst.

Task 1: 
src/iterators/two_merge_iterator.rs

We need this merge iterator for ssts, because we need a merge iterator for memtable + SST, walking both and return results in sorted order. Basically, we need to compare the current key of both and increment if one is bigger/smaller, just one is catching up

Task 2: Read Path - Scan
src/lsm_iterator.rs
src/lsm_storage.rs

Now we ensure that the lsm iterator works. Since SST iterator doesn't know the end, we need to add a bounds to check if the key is at the end. (in the case if the end bound is unbounded). In addition, we need to keep track of prev key to ensure that we don't that get the same key again (SST may give same key again...).

Task 3: Read Path - Get
src/lsm_storage.rs

Finally call scan to iterate over every element. Probe the memtable, and then the SSTs.

# Questions

