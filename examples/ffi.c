#include<stdlib.h>
#include<stdint.h>
#include<assert.h>
#include "../include/voluntary_servitude.h"

int main(int argc, char **argv) {
	vsread_t * const vsread = vsread_new();
	assert(vsread_len(vsread) == 0);

	const uint8_t data[3] = {12, 25, 89};
	vsread_append(vsread, (void *) data);
	assert(vsread_len(vsread) == 1);
	vsread_append(vsread, (void *) (data + 1));
	assert(vsread_len(vsread) == 2);

	vsread_iter_t * const iter = vsread_iter(vsread);
	vsread_clear(vsread);
	assert(vsread_len(vsread) == 0);

	assert(vsread_iter_len(iter) == 2);
	assert(vsread_iter_index(iter) == 0);
	assert(*(uint8_t *) vsread_iter_next(iter) == 12);
	assert(vsread_iter_index(iter) == 1);
	assert(*(uint8_t *) vsread_iter_next(iter) == 25);
	assert(vsread_iter_index(iter) == 2);
	assert(vsread_iter_next(iter) == NULL);
	assert(vsread_iter_next(iter) == NULL);
	assert(vsread_iter_index(iter) == 2);
	vsread_iter_destroy(iter);

	vsread_destroy(vsread);

	vsread_iter_t * const iter2 = vsread_iter(vsread);
	assert(vsread_iter_len(iter2) == 0);
	assert(vsread_iter_next(iter2) == NULL);
	vsread_iter_destroy(iter2);

	(void) argc;
	(void) argv;
	return 0;
}
