#include<pthread.h>
#include<assert.h>
#include<stdlib.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

const unsigned int num_producers = 4;
const unsigned int num_consumers = 8;

const unsigned int num_producer_values = 1000;
const unsigned int data[3] = {12, 25, 89};
const size_t last_index = sizeof(data) / sizeof(data[0]) - 1;

void * producer(void *);
void * consumer(void *);

int main(int argc, char** argv) {
    // You are responsible for making sure 'vs' exists while accessed
    vs_t * vs = vs_new();
	uint8_t thread = 0;
    pthread_attr_t attr;
    pthread_t consumers[num_consumers],
              producers[num_producers];

    if (pthread_attr_init(&attr) != 0) {
        fprintf(stderr, "Failed to initialize pthread arguments.\n");
        exit(-1);
    }

    // Creates producer threads
    for (thread = 0; thread < num_producers; ++thread) {
        if (pthread_create(&producers[thread], &attr, &producer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create producer thread %d.\n", thread);
            exit(-2);
        }

    }

    // Creates consumers threads
    for (thread = 0; thread < num_consumers; ++thread) {
        if (pthread_create(&consumers[thread], &attr, &consumer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create consumer thread %d.\n", thread);
            exit(-3);
        }
    }

    // Join all threads, ensuring vs_t* is not used anymore
    for (thread = 0; thread < num_producers; ++thread) {
        pthread_join(producers[thread], NULL);
    }
    for (thread = 0; thread < num_consumers; ++thread) {
        pthread_join(consumers[thread], NULL);
    }

    // Never forget to free the memory allocated through rust
    assert(vs_destroy(vs) == 0);

    printf("Multi thread example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}

void * producer(void * vs){
    unsigned int index;
    for (index = 0; index < num_producer_values; ++index) {
        assert(vs_append(vs, (void *) &data[index % last_index]) == 0);
    }
    return NULL;
}

void * consumer(void * vs) {
    const unsigned int total_values = num_producers * num_producer_values;
    unsigned int values = 0;

    while (values < total_values) {
        unsigned int sum = (values = 0);
        vs_iter_t * iter = vs_iter(vs);
        void * value;

        while ((value = vs_iter_next(iter)) != NULL) {
            ++values;
            sum += *(unsigned int *) value;
        }
        printf("Consumer counts %d elements summing %d.\n", values, sum);

        assert(vs_iter_destroy(iter) == 0);
    }
    return NULL;
}
