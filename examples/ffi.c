#include<assert.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

int main(int argc, char **argv) {
    // Rust allocates memory through malloc
    vsread_t * vsread = vsread_new();

    // Current vsread_t length
    // Be careful with data-races since the value, when used, may not be true anymore
    assert(vsread_len(vsread) == 0);

    const unsigned int data[2] = {12, 25};
    // Inserts void pointer to data to end of vsread_t
    vsread_append(vsread, (void *) &data[0]);
    vsread_append(vsread, (void *) &data[1]);

    // Creates a one-time lock-free iterator based on vsread_t
    vsread_iter_t * iter = vsread_iter(vsread);
    // Index changes as you iter through vsread_iter_t
    assert(vsread_iter_index(iter) == 0);

    // Clearing vsread_t, doesn't change existing iterators
    vsread_clear(vsread);
    assert(vsread_len(vsread) == 0);
    assert(vsread_iter_len(iter) == 2);

    assert(*(unsigned int *) vsread_iter_next(iter) == 12);
    assert(vsread_iter_index(iter) == 1);
    assert(*(unsigned int *) vsread_iter_next(iter) == 25);
    assert(vsread_iter_index(iter) == 2);

    assert(vsread_iter_next(iter) == NULL);
    assert(vsread_iter_index(iter) == 2);
    assert(vsread_iter_len(iter) == 2);

    // Never forget to free vsread_iter_t
    assert(vsread_iter_destroy(iter) == 0);

    // Create updated vsread_iter_t
    vsread_iter_t * iter2 = vsread_iter(vsread);

    // Never forget to free vsread_t
    assert(vsread_destroy(vsread) == 0);

    // vsread_iter_t keeps existing after the original vsread_t is freed
    assert(vsread_iter_len(iter2) == 0);
    assert(vsread_iter_next(iter2) == NULL);
    assert(vsread_iter_index(iter2) == 0);
    assert(vsread_iter_destroy(iter2) == 0);

    printf("Single thread test ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}
