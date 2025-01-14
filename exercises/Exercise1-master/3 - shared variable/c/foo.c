// Compile with `gcc foo.c -Wall -std=gnu99 -lpthread`, or use the makefile
// The executable will be named `foo` if you use the makefile, or `a.out` if you use gcc directly

#include <pthread.h>
#include <stdio.h>

int i = 0;
pthread_mutex_t mutex1, mutex2;

// Note the return type: void*
void* incrementingThreadFunction(){
    // TODO: increment i 1_000_000 times
    pthread_mutex_lock(&mutex1);
    for(int j = 0; j < 100000; j++){
        i ++;
    }
    pthread_mutex_unlock(&mutex1);
    return NULL;
}

void* decrementingThreadFunction(){
    // TODO: decrement i 1_000_000 times
    pthread_mutex_lock(&mutex1);
    for(int j = 0; j < 100000; j++){
        i --;
    }
    pthread_mutex_unlock(&mutex1);
    return NULL;
}


int main(){
    // TODO: 
    // start the two functions as their own threads using `pthread_create`
    // Hint: search the web! Maybe try "pthread_create example"?
    pthread_t pinc; 
    pthread_t pdec; 
    pthread_mutex_init(&mutex1, NULL);
    pthread_mutex_init(&mutex2, NULL);


    // Creating a new thread 
    
    pthread_create(&pinc, NULL, &incrementingThreadFunction, NULL);     
    pthread_create(&pdec, NULL, &decrementingThreadFunction, NULL); 
    
    printf("This line may be printed"
           " before thread terminates\n");   
    
    pthread_join(pinc, NULL);
    pthread_join(pdec, NULL);
    // TODO:
    // wait for the two threads to be done before printing the final result
    // Hint: Use `pthread_join`    
    printf("The magic number is: %d\n", i);

    pthread_mutex_destroy(&mutex1); 
    pthread_mutex_destroy(&mutex2); 

    return 0;
}


