#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <unistd.h>
#include <string.h>
#include "librespot-c/librespot-c.h"
static int connect_unused(const char *host, unsigned short port) { (void)host; (void)port; return -1; }
static void disconnect_unused(int fd) { (void)fd; }
static void log_unused(const char *fmt, ...) { (void)fmt; }
static void dump_unused(const char *msg, unsigned char *data, size_t size) { (void)msg; (void)data; (void)size; }
static void snapshot(const char *stage) {
  char line[256]; FILE *f = fopen("/proc/self/smaps_rollup", "r");
  printf("STAGE %s\n", stage);
  if (f) { while (fgets(line, sizeof(line), f)) {
    if (strncmp(line,"Rss:",4)==0 || strncmp(line,"Pss:",4)==0 || strncmp(line,"Private_Clean:",14)==0 || strncmp(line,"Private_Dirty:",14)==0) fputs(line,stdout);
  } fclose(f); }
  f = fopen("/proc/self/status", "r");
  if(f) { while (fgets(line,sizeof(line),f)) if(strncmp(line,"Threads:",8)==0) fputs(line,stdout); fclose(f); }
  fflush(stdout);
}
int main(void) {
  struct sp_sysinfo si = {0};
  struct sp_callbacks cb = { .tcp_connect=connect_unused, .tcp_disconnect=disconnect_unused, .logmsg=log_unused, .hexdump=dump_unused };
  snapshot("linked-before-init");
  if(librespotc_init(&si,&cb) != 0) { fprintf(stderr,"init failed: %s\n",librespotc_last_errmsg()); return 1; }
  sleep(2); snapshot("initialized-no-session-2s");
  sleep(10); snapshot("initialized-no-session-12s");
  librespotc_deinit(); snapshot("after-deinit");
  return 0;
}
