#include <perfmon/pfmlib_perf_event.h>
#include <errno.h>
#include <err.h>
#include <stdlib.h>
#include <string.h>
#include <locale.h>
#include <sys/time.h>

static int enabled = 0;
static int *perf_event_fds;
static struct perf_event_attr *perf_event_attrs;

void nop(void* anything) {

}

struct timeval* cur_time() {
    struct timeval* ret = (struct timeval*) malloc(sizeof(struct timeval));
    gettimeofday(ret, NULL);
    return ret;
}

double diff_in_ms(struct timeval* t_start, struct timeval* t_end) {
    return (double) (t_end->tv_usec - t_start->tv_usec) / 1000 + (double) (t_end->tv_sec - t_start->tv_sec) * 1000;
}

void sysPerfEventInit(int numEvents)
{
  int ret = pfm_initialize();
  if (ret != PFM_SUCCESS) {
    errx(1, "error in pfm_initialize: %s", pfm_strerror(ret));
  }

  perf_event_fds = (int*)calloc(numEvents, sizeof(int));
  if (!perf_event_fds) {
    errx(1, "error allocating perf_event_fds");
  }
  perf_event_attrs = (struct perf_event_attr *)calloc(numEvents, sizeof(struct perf_event_attr));
  if (!perf_event_attrs) {
    errx(1, "error allocating perf_event_attrs");
  }
  int i = 0;
  for(; i < numEvents; i++) {
    perf_event_attrs[i].size = sizeof(struct perf_event_attr);
  }
  enabled = 1;
}

void sysPerfEventCreate(int id, const char *eventName)
{
  struct perf_event_attr *pe = (perf_event_attrs + id);
  int ret = pfm_get_perf_event_encoding(eventName, PFM_PLM3, pe, NULL, NULL);
  if (ret != PFM_SUCCESS) {
    errx(1, "error creating event %d '%s': %s\n", id, eventName, pfm_strerror(ret));
  }
  pe->read_format = PERF_FORMAT_TOTAL_TIME_ENABLED | PERF_FORMAT_TOTAL_TIME_RUNNING;
  pe->disabled = 1;
  pe->inherit = 1;
  perf_event_fds[id] = perf_event_open(pe, 0, -1, -1, 0);
  if (perf_event_fds[id] == -1) {
    err(1, "error in perf_event_open for event %d '%s'", id, eventName);
  }
}

void sysPerfEventEnable()
{
  if (enabled) {
    if (prctl(PR_TASK_PERF_EVENTS_ENABLE)) {
      err(1, "error in prctl(PR_TASK_PERF_EVENTS_ENABLE)");
    }
  }
}

void sysPerfEventDisable()
{
  if (enabled) {
    if (prctl(PR_TASK_PERF_EVENTS_DISABLE)) {
      err(1, "error in prctl(PR_TASK_PERF_EVENTS_DISABLE)");
    }
  }
}

#define RAW_COUNT    0
#define TIME_ENABLED 1
#define TIME_RUNNING 2

long long sysPerfEventRead(int id) {
  long long readBuffer[3];

  // read into readBuffer
  size_t expectedBytes = 3 * sizeof(long long);
  int ret = read(perf_event_fds[id], readBuffer, expectedBytes);
  if (ret < 0) {
    err(1, "error reading event: %s", strerror(errno));
  }
  if (ret != expectedBytes) {
    errx(1, "read of perf event did not return 3 64-bit values");
  }

  // scale if needed
  if (readBuffer[TIME_ENABLED] != readBuffer[TIME_RUNNING]) {
    double scaleFactor;
    if (readBuffer[TIME_RUNNING] == 0) {
      scaleFactor = 0;
    } else {
      scaleFactor = readBuffer[TIME_ENABLED] / readBuffer[TIME_RUNNING];
    }

    readBuffer[RAW_COUNT] = (long long) (readBuffer[RAW_COUNT] * scaleFactor);
  }

  return readBuffer[RAW_COUNT];
}

struct PerfEvents {
    int n_events;
    const char** events;
    long long* last_read;
    long long* diff;
};

#define N_EVENTS 4
struct PerfEvents* start_perf_events() {
    struct PerfEvents* ret = (struct PerfEvents*) malloc(sizeof(struct PerfEvents));

    ret->n_events = N_EVENTS;
    ret->events = (const char**) malloc(sizeof(const char*) * N_EVENTS);

    ret->events[0] = "instructions";
    ret->events[1] = "cycles";
    ret->events[2] = "uops_retired:all";
    ret->events[3] = "idq:ms_uops";

//    ret->events[0] = "L2_RQSTS:ALL_PF";
//    ret->events[1] = "L2_RQSTS:REFERENCES";
//    ret->events[2] = "L2_RQSTS:L2_PF_MISS";
//    ret->events[3] = "L2_RQSTS:MISS";

//    ret->events[0] = "L2_RQSTS:CODE_RD_MISS";
//    ret->events[1] = "L2_RQSTS:CODE_RD_HIT";
//    ret->events[2] = "L2_RQSTS:REFERENCES";
//    ret->events[3] = "L2_RQSTS:MISS";

//    ret->events[2] = "l1d:replacement";
//    ret->events[0] = "LSD:UOPS";
//    ret->events[1] = "IDQ:DSB_UOPS";
//    ret->events[2] = "IDQ:MS_UOPS";
//    ret->events[3] = "IDQ:EMPTY";

//    ret->events[0] = "mem_uops_retired:all_loads";
//    ret->events[1] = "mem_uops_retired:all_stores";
//    ret->events[2] = "mem_uops_retired:stlb_miss_loads";
//    ret->events[3] = "mem_uops_retired:stlb_miss_stores";

//    ret->events[0] = "icache:miss";
//    ret->events[1] = "llc_misses";
//    ret->events[2] = "mem_load_uops_retired:l1_miss";
//    ret->events[3] = "dtlb_store_misses:miss_causes_a_walk";


//    ret->events[0] = "instructions";
//    ret->events[1] = "cycles";
//    ret->events[2] = "uops_retired:all";

//    ret->events[0] = "instructions";
//    ret->events[1] = "cycles";
//    ret->events[2] = "cache-misses";
//    ret->events[4] = "L1-dcache-loads";
//    ret->events[5] = "L1-dcache-load-misses";
//    ret->events[6] = "L1-dcache-stores";
//    ret->events[7] = "L1-dcache-store-misses";
//    ret->events[8] = "L1-dcache-prefetch-misses";
//    ret->events[3] = "UOPS_ISSUED:SLOW_LEA";
//    ret->events[10]= "LSD:UOPS";
//    ret->events[11]= "IDQ:DSB_UOPS";
//    ret->events[12]= "IDQ:ALL_MITE_UOPS";

    sysPerfEventInit(N_EVENTS);
    int i = 0;
    for(; i < N_EVENTS; i++)
        sysPerfEventCreate(i, ret->events[i]);

    sysPerfEventEnable();

    ret->last_read = (long long*) malloc(sizeof(long long) * N_EVENTS);
    ret->diff = (long long*) malloc(sizeof(long long) * N_EVENTS);

    return ret;
}

void perf_read_values(struct PerfEvents* events) {
    long long *last_read = events->last_read;

    events->last_read = (long long*) malloc(sizeof(long long) * N_EVENTS);
    int i = 0;
    for(; i < N_EVENTS; i++) {
        events->last_read[i] = sysPerfEventRead(i);
        events->diff[i] = events->last_read[i] - last_read[i];
    }

    free(last_read);
}

void perf_print(struct PerfEvents* events) {
    setlocale(LC_NUMERIC, "");
    printf("---perf results---\n");
    int i = 0;
    for(; i < N_EVENTS; i++)
        printf("%-30s : %'lld\n", events->events[i], events->diff[i]);
    printf("------------------\n");
}
