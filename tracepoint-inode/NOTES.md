# Questions

* Why sequential writes have more `ext4_mark_inode_dirty` calls than random writes?
* What are the difference between `fallocate` flags `FALLOC_FL_KEEP_SIZE` and `FALLOC_ZERO_RANGE`? What are their impacts on inode operations and data blocks?
* What are the performance implications of events like `ext4_mark_inode_dirty` and `ext4_allocate_blocks`?
* Why double writes are always faster than `fallocate`?