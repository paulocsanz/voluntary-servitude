#include<pthread.h>
#include<assert.h>
#include<stdlib.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

const unsigned int num_consumers = 8;
const unsigned int num_producers = 4;
const unsigned int num_threads = 12;

const unsigned int num_producer_values = 10000000;
const unsigned int data = 3;

void * producer(void *);
void * consumer(void *);

int main(int argc, char** argv) {
    // You are responsible for making sure 'vs' exists while accessed
    vs_t * vs = vs_new();
    uint8_t thread = 0;
    pthread_attr_t attr;
    pthread_t threads[num_threads];

    if (pthread_attr_init(&attr) != 0) {
        fprintf(stderr, "Failed to initialize pthread arguments.\n");
        exit(-1);
    }

    // Creates producer threads
    for (thread = 0; thread < num_producers; ++thread) {
        if (pthread_create(&threads[thread], &attr, &producer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create producer thread %d.\n", thread);
            exit(-2);
        }

    }

    // Creates consumers threads
    for (thread = 0; thread < num_consumers; ++thread) {
        if (pthread_create(&threads[num_producers + thread], &attr, &consumer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create consumer thread %d.\n", thread);
            exit(-3);
        }
    }

    // Join all threads, ensuring vs_t* is not used anymore
    for (thread = 0; thread < num_threads; ++thread) {
        pthread_join(threads[thread], NULL);
    }

    // Never forget to free the memory allocated through the lib
    assert(vs_destroy(vs) == 0);

    printf("Multi-thread C example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}

void * producer(void * vs){
    unsigned int index;
    for (index = 0; index < num_producer_values; ++index) {
        assert(vs_append(vs, (void *) &data) == 0);
    }
    return NULL;
}

void * consumer(void * vs) {
    const unsigned int total_values = num_producers * num_producer_values;
    unsigned int values = 0;

    while (values < total_values) {
        vs_iter_t * iter = vs_iter(vs);
        void * value;

        values = 0;
        while ((value = vs_iter_next(iter)) != NULL) {
            ++values;
        }
        printf("%d elements\n", values);

        // Never forget to free the memory allocated through the lib
        assert(vs_iter_destroy(iter) == 0);
    }
    return NULL;
}
