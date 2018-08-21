#include<pthread.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

const unsigned int num_producers = 4;
const unsigned int num_consumers = 8;

const unsigned int num_producer_values = 1000;
const unsigned int data[3] = {12, 25, 89};

void* producer();
void* consumer();

int main(int argc, char** argv)
{
    // Rust allocates memory through malloc
    vsread_t * const vsread = vsread_new();
    unsigned int current_thread = 0;
    pthread_attr_t attr;
    pthread_t consumers[num_consumers],
              producers[num_producers];

    if (pthread_attr_init(&attr) != 0) {
        fprintf(stderr, "Failed to initialize pthread arguments.\n");
        exit(-1);
    }

    // Creates producer threads
    for (current_thread = 0; current_thread < num_producers; ++current_thread) {
        if (pthread_create(&producers[current_thread], &attr, &producer, (void *) vsread) != 0) {
            fprintf(stderr, "Failed to create producer thread %d.\n", current_thread);
            exit(-2);
        }

    }
    
    // Creates consumers threads
    for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
        if (pthread_create(&consumers[current_thread], &attr, &consumer, (void *) vsread) != 0) {
            fprintf(stderr, "Failed to create consumer thread %d.\n", current_thread);
            exit(-3);
        }
    }

    // Join all threads, ensuring vsread_t* is not used anymore
    for (current_thread = 0; current_thread < num_producers; ++current_thread) {
        pthread_join(producers[current_thread], NULL);
    }
    for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
        pthread_join(consumers[current_thread], NULL);
    }

    // Never forget to free the memory allocated through rust
    vsread_destroy(vsread);

    (void) argc;
    (void) argv;
    return 0;
}


void * producer(void * const vsread){
    unsigned int index;
    for (index = 0; index < num_producer_values; ++index) {
        vsread_append(vsread, (void *) (data + (index % 2)));
    }
    return NULL;
}

void * consumer(void * const vsread) {
    const unsigned int total_values = num_producers * num_producer_values;
    unsigned int values = 0;
    while (values < total_values) {
        unsigned int sum = (values = 0);
        vsread_iter_t * const iter = vsread_iter(vsread);
        const void * value;
        while ((value = vsread_iter_next(iter)) != NULL) {
            ++values;
            sum += *(unsigned int *) value;
        }
        printf("Consumer counts %d elements summing %d.\n", values, sum);

        vsread_iter_destroy(iter);
    }
    return NULL;
}
