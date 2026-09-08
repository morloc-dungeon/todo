# Build/test conventions for a morloc-dungeon example.
#
# dungeon-master drives exactly these three targets:
#
#   make build   build the program; nonzero exit on any build failure
#   make test    run it and diff against test/exp.txt; nonzero exit on mismatch
#   make clean   remove build products
#
# `test` depends on `build`, so `make test` builds and checks in one step.
# `diff` exits nonzero on any mismatch, which is what marks the test as failed.

EXE := todo
SRC := src/main.loc

.PHONY: build test clean

build:
	morloc make -o $(EXE) $(SRC)

test: build
	: > test/obs.txt
	./$(EXE) add 3 4 >> test/obs.txt
	./$(EXE) sumList '[1,2,3,4]' >> test/obs.txt
	diff -u test/exp.txt test/obs.txt

clean:
	rm -rf $(EXE) $(EXE)-build test/obs.txt
