Exercise 1 - Theory questions
-----------------------------

### Concepts

What is the difference between *concurrency* and *parallelism*?
> Concurency is rapid change of running threads, whereas parallelism is simuntinously running tasks. Concurency can occur on a single core with overlapping start, run and end. Parallelism needs multiple cores.

What is the difference between a *race condition* and a *data race*? 
> Data race is when two or more tasks access the same memory (data) and at least one task is preforming write to the memory.
Race condition is errors occuring due to the timing of tasks, and is a semantic error. Can be a data race, not nececcary
 
*Very* roughly - what does a *scheduler* do, and how does it do it?
> A scheduler assign resources to preforming tasks(threads/procecees)


### Engineering

Why would we use multiple threads? What kinds of problems do threads solve?
> Threads solve the problem of not solving problems when no other task is running

Some languages support "fibers" (sometimes called "green threads") or "coroutines"? What are they, and why would we rather use them over threads?
> Fibers are abstrahaded threads running innenvironments such as virual machines, that allow for more control of scheduling

Does creating concurrent programs make the programmer's life easier? Harder? Maybe both?
> Depending on the implementation a programmers life can become either easier or harder. If concurency is acheived by complexity hardship is very possible.

What do you think is best - *shared variables* or *message passing*?
> Not knowing to much i imagine message passing is best bc its simplicity.


