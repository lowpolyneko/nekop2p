# Testing
## Performance Results
The utilization of an async model with a pre-allocated thread pool turns out to
be quite effective for this use-case. Below is a baseline benchmark running 500
search requests sequentially.

![](profile-1-scatter.png)

Overall queries average a response time of `37.28` µs over `localhost` which is
quite impressive. This is around the range of plain TCP RTT on `localhost`[^1].
This means that the overhead of Tarpc and Tokio is negligible when considering
sequential connections.

![](profile-1-hist.png)

We can see the distribution of response times follows a normal distribution
aswell.

## 5 Concurrent
With a concurrent load, the average jumps to `90.58` µs over `localhost`. This
means we see only a doubling of response time in the *microsecond* range with
additional clients.

![](profile-5-scatter.png)

The distribution of response times remain fairly normal aswell.

![](profile-5-hist.png)

## 10 Concurrent
With a doubled concurrent load, the average jumps to `123.51` µs over
`localhost`. While I do not have further data to analyze, this indicates that
response time with the async model of `nekop2p` scales logarithmically.

![](profile-10-scatter.png)

The distribution of response times continues to remain fairly normal.

![](profile-10-hist.png)

## Note on Response Time Spike with Beginning Requests
In the concurrent examples, initial responses incur a significantly higher cost
than subsequent runs. I attribute this to the locking scheme used by the
underlying `HashMap` (the `dashmap` crate). Dashmap's implementation allows for
better concurrency than simply wrapping a `HashMap<K,V>` around a `Mutex<T>`,
and I believe that it amortizes as requests increase in-spite of lock
contention between threads with multiple clients querying.

[^1]: http://doc-ok.org/?p=1985
