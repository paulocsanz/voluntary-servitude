LIBDIR=../target/release

all: ffi.c $(LIBDIR)/libvoluntary_servitude.so
	$(CC) -Wall -Wextra -Wpedantic ffi.c -L$(LIBDIR) -lvoluntary_servitude -o ffi


$(LIBDIR)/libvoluntary_servitude.so:
	cargo build --release

clean:
	rm -rf target
