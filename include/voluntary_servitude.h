typedef struct vsread_S vsread_t;
typedef struct vsread_iter_S vsread_iter_t;

extern vsread_t* vsread_new(void);
extern vsread_iter_t* vsread_iter(const vsread_t * const);
extern size_t vsread_len(const vsread_t * const);
extern void vsread_append(vsread_t * const, const void * const);
extern void vsread_clear(vsread_t * const);
extern void vsread_destroy(vsread_t * const);

extern void* vsread_iter_next(vsread_iter_t * const);
extern size_t vsread_iter_len(const vsread_iter_t * const);
extern size_t vsread_iter_index(const vsread_iter_t * const);
extern void vsread_iter_destroy(vsread_iter_t * const);
