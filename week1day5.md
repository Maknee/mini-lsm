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

Consider the case that a user has an iterator that iterates the whole storage engine, and the storage engine is 1TB large, so that it takes ~1 hour to scan all the data. What would be the problems if the user does so? (This is a good question and we will ask it several times at different points of the tutorial...)

It would lock the entire storage, so no additional data could be added.

Another popular interface provided by some LSM-tree storage engines is multi-get (or vectored get). The user can pass a list of keys that they want to retrieve. The interface returns the value of each of the key. For example, multi_get(vec!["a", "b", "c", "d"]) -> a=1,b=2,c=3,d=4. Obviously, an easy implementation is to simply doing a single get for each of the key. How will you implement the multi-get interface, and what optimizations you can do to make it more efficient? (Hint: some operations during the get process will only need to be done once for all keys, and besides that, you can think of an improved disk I/O interface to better support this multi-get interface).

For disk I/O, we try to find if any of the keys match and keep track of each one.
