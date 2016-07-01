#!/usr/bin/env python
import sys
import subprocess

def error(msg):
	print 'Error: ' + msg
	sys.exit

def monitor_invoke(command):
        print command
        run = subprocess.Popen(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
	while True:
		out = run.stdout.read(1)
		if out == '' and run.poll() != None:
			break;
		if out != '':
			sys.stdout.write(out)
			sys.stdout.flush()
	return True	

REMOTE_USER = 'yilin'
RSYNC_DIR = '/home/{}/rust_gc/immix_rust_new/'.format(REMOTE_USER)
MACHINE = 'rat.moma'

C_FLAGS = '-O3 -std=gnu99'
RUSTC_FLAGS = '--'

if len(sys.argv) < 4:
	error("e.g. ./run.py exhaust 500M 5");
	
feature = sys.argv[1]
all_features = ["exhaust", "initobj", "gcbench", "mark", "trace", "mt-gcbench"]
if feature not in all_features :
	error("unsupported feature")
	
heap = sys.argv[2]
invocations = sys.argv[3]

bm_args = ''
if len(sys.argv) > 4 and sys.argv[4] == '--':
	bm_args = ' '.join(sys.argv[5:])

print 'benchmark specific args: ' + bm_args

print '###feature = ' + feature
print '###heap = ' + heap
print '###invocations = ' + invocations

print 'rsync...'
remote_path = "{}@{}:{}".format(REMOTE_USER, MACHINE, RSYNC_DIR)
ret = subprocess.call(["rsync", "-rav", "-I", "--delete", "--exclude=target", "--exclude=.hg", ".", remote_path])
if ret != 0:
	error('failed to rsync')
	
for invocation in range(0, int(invocations)):
	print 'Invocation %d' % (invocation)
	print
	
	print '===Rust Version==='
	build = ''
	if feature == 'gcbench' or feature == 'mt-gcbench':
		build = 'ssh -t {}@{} "cd {}; rm -r target; cargo rustc --release --features \"{}\" {}; HEAP_SIZE={} N_GCTHREADS=7 ./target/release/immix-rust 7"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, feature, RUSTC_FLAGS, heap, bm_args)
	else:
		build = 'ssh -t {}@{} "cd {}; rm -r target; cargo rustc --release --features \"{}\" {}; HEAP_SIZE={} N_GCTHREADS=0 ./target/release/immix-rust"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, feature, RUSTC_FLAGS, heap, bm_args)
	succ = monitor_invoke(build)
	if not succ:
		error('failed to execute rust code')
	print

	print '===C Version==='
	build = 'ssh -t {}@{} "cd {}c_src; clang-3.7 {} *.c ../src/common/perf.c -lpfm -o bench -D{}; HEAP_SIZE={} ./bench {}"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, C_FLAGS, feature, heap, bm_args)
	succ = monitor_invoke(build)
	if not succ:
		error('failed to execute c code')
	print

	print '===Boehm GC==='	
	if feature == "exhaust":
		build = 'ssh -t {}@{} "cd {}../bdwgc/gctests/; clang-3.7 -I ../include exhaust.c ../*.o {} -DGC -DTHREAD_LOCAL_ALLOC -DPARALLEL_MARK -o exhaust -l pthread; GC_INITIAL_HEAP_SIZE={} GC_MAXIMUM_HEAP_SIZE={} ./exhaust"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, C_FLAGS, heap, heap)
	elif feature == "gcbench":
		build = 'ssh -t {}@{} "cd {}../bdwgc/gctests/; clang-3.7 -I ../include gcbench.c ../*.o {} -DGC -DTHREAD_LOCAL_ALLOC -DPARALLEL_MARK -o gcbench -l pthread; GC_INITIAL_HEAP_SIZE={} GC_MAXIMUM_HEAP_SIZE={} ./gcbench"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, C_FLAGS, heap, heap)
	elif feature == "mt-gcbench":
		build = 'ssh -t {}@{} "cd {}../bdwgc/gctests/; clang-3.7 -I ../include mt-gcbench.c ../*.o {} -DGC -DTHREAD_LOCAL_ALLOC -DPARALLEL_MARK -DNTHREADS=7 -o mt-gcbench -l pthread; GC_INITIAL_HEAP_SIZE={} GC_MAXIMUM_HEAP_SIZE={} ./mt-gcbench"'.format(REMOTE_USER, MACHINE, RSYNC_DIR, C_FLAGS, heap, heap, bm_args)	
	else:
		build = ''
		print '"{}" not applicable to bdwgc'.format(feature);
	if build != '':
		succ = monitor_invoke(build)
		if not succ:
			error('failed to execute boehm gc code')
		print
	
	print 'Invocation Finish'
