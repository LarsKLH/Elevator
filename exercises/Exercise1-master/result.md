Some results of foo.c using the naive implementation of threads:

The magic number is: -67110
The magic number is: 131507
The magic number is: -18946
The magic number is: -772874
The magic number is: -665186
The magic number is: 446027
The magic number is: -673082
The magic number is: -677601
The magic number is: 681971
The magic number is: 69918


This is because both threads try to modify the value of i and the order in witch the threading is interleaved changes the end result.



Some results of foo.c using a propper implementation:

The magic number has been increased to: 1
The magic number has been decreased to: 0
The magic number has been decreased to: -1
The magic number has been increased to: 0
The magic number has been decreased to: -1
The magic number has been decreased to: -2
The magic number has been increased to: -1
The magic number has been decreased to: -2
The magic number has been decreased to: -3
The magic number has been increased to: -2
The magic number has been decreased to: -3
The magic number has been increased to: -2
The magic number has been decreased to: -3
The magic number has been decreased to: -4
The magic number has been increased to: -3
The magic number has been decreased to: -4
The magic number has been increased to: -3
The magic number has been increased to: -2
The magic number has been increased to: -1
The magic number has been increased to: 0
The magic number is: 0
The magic number has been increased to: 1
The magic number has been decreased to: 0
The magic number has been decreased to: -1
The magic number has been increased to: 0
The magic number has been decreased to: -1
The magic number has been decreased to: -2
The magic number has been increased to: -1
The magic number has been decreased to: -2
The magic number has been decreased to: -3
The magic number has been increased to: -2
The magic number has been decreased to: -3
The magic number has been decreased to: -4
The magic number has been increased to: -3
The magic number has been decreased to: -4
The magic number has been increased to: -3
The magic number has been decreased to: -4
The magic number has been increased to: -3
The magic number has been increased to: -2
The magic number has been increased to: -1
The magic number has been increased to: 0
The magic number is: 0
