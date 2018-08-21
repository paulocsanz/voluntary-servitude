LIBDIR=../dist
FLAGS=-Wall -Wextra -Wpedantic

default: all

all: multithread.c ffi.c $(LIBDIR)/libvoluntary_servitude.so
	$(CC) $(FLAGS) ffi.c -L$(LIBDIR) -lvoluntary_servitude -o ffi
	$(CC) $(FLAGS) multithread.c -L$(LIBDIR) -lpthread -lvoluntary_servitude -o multithread


$(LIBDIR)/libvoluntary_servitude.so:
	cargo build --release
	cp ../target/release/libvoluntary_servitude.so $(LIBDIR)

test:
	make && LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./ffi
	make && LD_LIBRARY_PATH=$(LIBDIR):$LD_LIBRARY_PATH ./multithread

clean:
	rm -rf ../target ffi multithread
