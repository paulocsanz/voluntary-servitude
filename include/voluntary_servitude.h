#ifndef VOLUNTARY_SERVITUDE_H
#define VOLUNTARY_SERVITUDE_H

#include<stdint.h>

/*
typedef struct {
	usize size;


#define DEFINE_VS_TYPE(content_type__) \
typedef struct {                                            \
	usize size;                                             \
	uint8_t[sizeof(content_type__)]
    content_type__ content;                                 \
} TYPE_##content_type_name__

DEFINE_FOO_TYPE(char, foo_t);
*/

typedef struct vs_S vs_t;
typedef struct vs_iter_S vs_iter_t;

extern vs_t * vs_new(void);
extern vs_iter_t * vs_iter(vs_t * const);
extern size_t vs_len(const vs_t * const);
extern uint8_t vs_append(vs_t * const, const void * const);
extern uint8_t vs_clear(vs_t * const);
extern uint8_t vs_destroy(vs_t * const);

extern void * vs_iter_next(vs_iter_t * const);
extern size_t vs_iter_len(const vs_iter_t * const);
extern size_t vs_iter_index(const vs_iter_t * const);
extern uint8_t vs_iter_destroy(vs_iter_t * const);

#endif
