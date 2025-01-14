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
> Threads solve the innefficiency of some tasks requirering inactivity (waiting / reading from disk) and enabling several different prosecess to "run simuntaniusly" 

Some languages support "fibers" (sometimes called "green threads") or "coroutines"? What are they, and why would we rather use them over threads?
> Fibers are abstracted threads running in environments such as virual machines, that allow for more control of scheduling

Does creating concurrent programs make the programmer's life easier? Harder? Maybe both?
> Depending on the implementation a programmers life can become either easier or harder. Concurency can dratically simplify a problem though it can also add a lot of complexity and hardship

What do you think is best - *shared variables* or *message passing*?
> From our standpoint we prefer message passing in an abstracted sense as it simplifies the program structure. Though shared variables may be prefarable in cases like global states.


