#include<assert.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

int main(int argc, char **argv) {
    // Rust allocates memory through malloc
    vsread_t * const vsread = vsread_new();

    // Current vsread_t length
    // Be careful with data-races since the value, when used, may not be true anymore
    assert(vsread_len(vsread) == 0);

    const unsigned int data[2] = {12, 25};
    // Inserts void pointer to data to end of vsread_t
    vsread_append(vsread, (void *) data);
    assert(vsread_len(vsread) == 1);

    vsread_append(vsread, (void *) (data + 1));
    assert(vsread_len(vsread) == 2);

    // Creates a one-time lock-free iterator based on vsread_t
    vsread_iter_t * const iter = vsread_iter(vsread);
    // Index changes as you iter through vsread_iter_t
    assert(vsread_iter_index(iter) == 0);

    // Clears vsread_t, doesn't change existing iterators
    vsread_clear(vsread);
    assert(vsread_len(vsread) == 0);
    assert(vsread_iter_len(iter) == 2);

    assert(*(unsigned int *) vsread_iter_next(iter) == 12);
    assert(vsread_iter_index(iter) == 1);

    assert(*(unsigned int *) vsread_iter_next(iter) == 25);
    assert(vsread_iter_index(iter) == 2);

    // Next can be called after there are no more elements (NULL pointer returned), but nothing happens
    assert(vsread_iter_next(iter) == NULL);
    assert(vsread_iter_next(iter) == NULL);
    assert(vsread_iter_index(iter) == 2);

    // Never forget to free vsread_iter_t
    vsread_iter_destroy(iter);

    // Create updated vsread_iter_t
    vsread_iter_t * const iter2 = vsread_iter(vsread);

    // Never forget to free vsread_t
    vsread_destroy(vsread);

    assert(vsread_iter_len(iter2) == 0);
    assert(vsread_iter_next(iter2) == NULL);

    vsread_iter_destroy(iter2);

    // Unused arguments
    (void) argc;
    (void) argv;
	printf("Single thread test ended (ffi.c)\n");
    return 0;
}
