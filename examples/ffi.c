#include<assert.h>
#include<stdlib.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

int main(int argc, char **argv) {
    // You are responsible for making sure 'vs' exists while accessed
    vs_t * vs = vs_new(&free);

    // Current vs_t length
    // Be careful with race conditions since the value, when used, may not be true anymore
    assert(vs_len(vs) == 0);

    uint8_t * const data = malloc(sizeof(*data));
    data[0] = 12;
    data[1] = 25;

    // Inserts void pointer to data to end of vs_t
    vs_append(vs, &data[0]);
    vs_append(vs, &data[1]);

    // Creates a one-time lock-free iterator based on vs_t
    vs_iter_t * iter = vs_iter(vs);

    // Clearing vs_t, doesn't change existing iterators
    vs_clear(vs);
    assert(vs_len(vs) == 0);
    assert(vs_iter_len(iter) == 2);

    assert(*(uint8_t *) vs_iter_next(iter) == 12);
    // Index changes as you iter through vs_iter_t
    assert(vs_iter_index(iter) == 1);
    assert(*(uint8_t *) vs_iter_next(iter) == 25);
    assert(vs_iter_index(iter) == 2);

    assert(vs_iter_next(iter) == NULL);
    assert(vs_iter_index(iter) == 2);
    // Index doesn't increase after it gets equal to 'len'
    // Length also is unable to increase after iterator is consumed
    assert(vs_iter_index(iter) == vs_iter_len(iter));

    // Never forget to free vs_iter_t
    assert(vs_iter_destroy(iter) == 0);

    // Create updated vs_iter_t
    vs_iter_t * iter2 = vs_iter(vs);

    // Never forget to free vs_t (NULL since it's static data, so we won't free it)
    assert(vs_destroy(vs) == 0);

    // vs_iter_t keeps existing after the original vs_t is freed (or cleared)
    assert(vs_iter_len(iter2) == 0);
    assert(vs_iter_next(iter2) == NULL);
    assert(vs_iter_index(iter2) == 0);

    assert(vs_iter_destroy(iter2) == 0);

    printf("Single thread example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}
