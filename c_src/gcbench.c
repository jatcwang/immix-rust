// This is adapted from a benchmark written by John Ellis and Pete Kovac
// of Post Communications.
// It was modified by Hans Boehm of Silicon Graphics.
// Translated to C++ 30 May 1997 by William D Clinger of Northeastern Univ.
// Translated to C 15 March 2000 by Hans Boehm, now at HP Labs.
//
//      This is no substitute for real applications.  No actual application
//      is likely to behave in exactly this way.  However, this benchmark was
//      designed to be more representative of real applications than other
//      Java GC benchmarks of which we are aware.
//      It attempts to model those properties of allocation requests that
//      are important to current GC techniques.
//      It is designed to be used either to obtain a single overall performance
//      number, or to give a more detailed estimate of how collector
//      performance varies with object lifetimes.  It prints the time
//      required to allocate and collect balanced binary trees of various
//      sizes.  Smaller trees result in shorter object lifetimes.  Each cycle
//      allocates roughly the same amount of memory.
//      Two data structures are kept around during the entire process, so
//      that the measured performance is representative of applications
//      that maintain some live in-memory data.  One of these is a tree
//      containing many pointers.  The other is a large array containing
//      double precision floating point numbers.  Both should be of comparable
//      size.
//
//      The results are only really meaningful together with a specification
//      of how much memory was used.  It is possible to trade memory for
//      better time performance.  This benchmark should be run in a 32 MB
//      heap, though we don't currently know how to enforce that uniformly.
//
//      Unlike the original Ellis and Kovac benchmark, we do not attempt
//      measure pause times.  This facility should eventually be added back
//      in.  There are several reasons for omitting it for now.  The original
//      implementation depended on assumptions about the thread scheduler
//      that don't hold uniformly.  The results really measure both the
//      scheduler and GC.  Pause time measurements tend to not fit well with
//      current benchmark suites.  As far as we know, none of the current
//      commercial Java implementations seriously attempt to minimize GC pause
//      times.

#include <stdio.h>
#include <stdlib.h>
#include <sys/time.h>

#include "common.h"
#include "immix.h"
#include <stdint.h>

//  These macros were a quick hack for the Macintosh.
//
//  #define currentTime() clock()
//  #define elapsedTime(x) ((1000*(x))/CLOCKS_PER_SEC)

#define currentTime() stats_rtclock()
#define elapsedTime(x) (x)

/* Get the current time in milliseconds */

unsigned
stats_rtclock( void )
{
  struct timeval t;
  struct timezone tz;

  if (gettimeofday( &t, &tz ) == -1)
    return 0;
  return (t.tv_sec * 1000 + t.tv_usec / 1000);
}

static const int kStretchTreeDepth    = 18;      // about 16Mb
static const int kLongLivedTreeDepth  = 16;  // about 4Mb
static const int kArraySize  = 500000;  // about 4Mb
static const int kMinTreeDepth = 4;
static const int kMaxTreeDepth = 16;

typedef struct Node0_struct {
    uint64_t invisible_hdr;
    struct Node0_struct * left;
    struct Node0_struct * right;
    int i, j;
} Node0;

void init_Node(Node0* me, Node0* l, Node0* r) {
    me -> left = l;
    me -> right = r;
}

// Nodes used by a tree of a given size
static int TreeSize(int i) {
    return ((1 << (i + 1)) - 1);
}

// Number of iterations to use for a given tree depth
static int NumIters(int i) {
    return 2 * TreeSize(kStretchTreeDepth) / TreeSize(i);
}

// Build tree top down, assigning to older objects.
static void Populate(int iDepth, Node0* thisNode, ImmixMutatorLocal* mutator) {
    if (iDepth<=0) {
        return;
    } else {
        iDepth--;
        thisNode->left = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
        thisNode->right = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
        Populate (iDepth, thisNode->left, mutator);
        Populate (iDepth, thisNode->right, mutator);
    }
}

// Build tree bottom-up
static Node0* MakeTree(int iDepth, ImmixMutatorLocal* mutator) {
    Node0* result;
    if (iDepth<=0) {
        result = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
        /* result is implicitly initialized in both cases. */
        return result;
    } else {
        Node0* left = MakeTree(iDepth-1, mutator);
        Node0* right = MakeTree(iDepth-1, mutator);
        result = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
        init_Node(result, left, right);
        return result;
    }
}

static void PrintDiagnostics() {

}

static void TimeConstruction(int depth, ImmixMutatorLocal* mutator) {
    long    tStart, tFinish;
    int     iNumIters = NumIters(depth);
    Node0*  tempTree;
    int     i;

    printf("Creating %d trees of depth %d\n", iNumIters, depth);

    tStart = currentTime();
    for (i = 0; i < iNumIters; ++i) {
        tempTree = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
        Populate(depth, tempTree, mutator);
        tempTree = 0;
    }
    tFinish = currentTime();
    printf("\tTop down construction took %ld msec\n",
           elapsedTime(tFinish - tStart));

    tStart = currentTime();
    for (i = 0; i < iNumIters; ++i) {
            tempTree = MakeTree(depth, mutator);
            tempTree = 0;
    }
    tFinish = currentTime();
    printf("\tBottom up construction took %ld msec\n",
           elapsedTime(tFinish - tStart));

}

void bench_start() {
    Node0*  root;
    Node0*  longLivedTree;
    Node0*  tempTree;
    long    tStart, tFinish;
    long    tElapsed;
    int     i, d;
    double  *array;

    gc_init();

    ImmixMutatorLocal* mutator = ImmixMutatorLocal_new(immix_space);

    printf("Garbage Collector Test\n");
    printf(" Live storage will peak at %ld bytes.\n\n",
           2 * sizeof(Node0) * TreeSize(kLongLivedTreeDepth) +
           sizeof(double) * kArraySize);
    printf(" Stretching memory with a binary tree of depth %d\n",
           kStretchTreeDepth);
    PrintDiagnostics();

    tStart = currentTime();

    // Stretch the memory space quickly
    tempTree = MakeTree(kStretchTreeDepth, mutator);
    tempTree = 0;

    // Create a long lived object
    printf(" Creating a long-lived binary tree of depth %d\n",
           kLongLivedTreeDepth);
    longLivedTree = (Node0*) ImmixMutatorLocal_alloc(mutator, sizeof(Node0), 8);
    Populate(kLongLivedTreeDepth, longLivedTree, mutator);

    // Create long-lived array, filling half of it
    printf(" Creating a long-lived array of %d doubles\n", kArraySize);
    array = malloc(kArraySize * sizeof(double));
    for (i = 0; i < kArraySize/2; ++i) {
        array[i] = 1.0/i;
    }
    PrintDiagnostics();

    for (d = kMinTreeDepth; d <= kMaxTreeDepth; d += 2) {
        TimeConstruction(d, mutator);
    }

    if (longLivedTree == 0 || array[1000] != 1.0/1000)
        fprintf(stderr, "Failed\n");
    // fake reference to LongLivedTree
    // and array
    // to keep them from being optimized away

    tFinish = currentTime();
    tElapsed = elapsedTime(tFinish-tStart);
    PrintDiagnostics();
    printf("Completed in %ld msec\n", tElapsed);
}
