LIBDIR=../dist
FLAGS=-Wall -Wextra -Wpedantic

default: all

all:
	cargo build --release --features "system-alloc"
	cp ../target/release/libvoluntary_servitude.so $(LIBDIR)
	strip $(LIBDIR)/libvoluntary_servitude.so
	$(CC) $(FLAGS) ffi.c -L$(LIBDIR) -lvoluntary_servitude -o ffi
	$(CC) $(FLAGS) multithread.c -L$(LIBDIR) -lpthread -lvoluntary_servitude -o multithread

test:
	make
	cargo run --example singlethread
	cargo run --example multithread
	LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./ffi
	LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./multithread

clean:
	rm -rf ../target ffi multithread
