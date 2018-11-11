LIBDIR=../dist
FLAGS=-Wall -Wextra -Wpedantic -Werror -O3 -D_FORTIFY_SOURCE=2 -fvisibility=hidden -fstack-protector -fstack-protector-all

default: all

all:
	RUSTFLAGS=-Cpanic=abort cargo build --release
	cp ../target/release/libvoluntary_servitude.so $(LIBDIR)
	strip $(LIBDIR)/libvoluntary_servitude.so
	$(CC) $(FLAGS) ffi.c -L$(LIBDIR) -lvoluntary_servitude -o ffi
	$(CC) $(FLAGS) multithread.c -L$(LIBDIR) -lpthread -lvoluntary_servitude -o multithread

test:
	make
	cargo run --example singlethread --release
	cargo run --example multithread --release
	LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./ffi
	LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./multithread

clean:
	rm -rf ../target ffi multithread
