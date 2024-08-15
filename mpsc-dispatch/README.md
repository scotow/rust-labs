This lab was made to determine how the dispatch works within `crossbeam_channel`'s `mpsc` channals, in particular what happen when multiple receivers exist but only half are waiting for messages.

```shell
$ cargo run --release | sort | uniq -c | sort
   1 0
   1 2
   1 4
   1 6
   1 8
19201 5
19428 9
19587 1
20861 7
20918 3
```