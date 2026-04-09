// Where: scripts/bench/sqlite_commit_latency.c
// What: Measure SQLite single-row commit latency under fixed journal and synchronous settings.
// Why: VFS durability cost depends more on commit latency than on speedtest1's mixed workload shape.
#include <sqlite3.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/time.h>

static double now_us(void) {
  struct timeval tv;
  gettimeofday(&tv, NULL);
  return (double)tv.tv_sec * 1000000.0 + (double)tv.tv_usec;
}

static int compare_double(const void *left, const void *right) {
  const double lhs = *(const double *)left;
  const double rhs = *(const double *)right;
  if (lhs < rhs) return -1;
  if (lhs > rhs) return 1;
  return 0;
}

static double percentile(const double *values, int count, double pct) {
  int index = (int)((pct / 100.0) * (double)(count - 1));
  if (index < 0) index = 0;
  if (index >= count) index = count - 1;
  return values[index];
}

static void exec_or_die(sqlite3 *db, const char *sql) {
  char *err = NULL;
  if (sqlite3_exec(db, sql, NULL, NULL, &err) != SQLITE_OK) {
    fprintf(stderr, "sqlite error: %s\n", err ? err : "unknown");
    sqlite3_free(err);
    exit(1);
  }
}

int main(int argc, char **argv) {
  if (argc != 7) {
    fprintf(stderr, "usage: %s <db_path> <journal> <synchronous> <iterations> <payload_size> <raw_json>\n", argv[0]);
    return 1;
  }

  const char *db_path = argv[1];
  const char *journal = argv[2];
  const char *synchronous = argv[3];
  const int iterations = atoi(argv[4]);
  const int payload_size = atoi(argv[5]);
  const char *raw_json = argv[6];
  sqlite3 *db = NULL;
  sqlite3_stmt *stmt = NULL;
  double *latencies = calloc((size_t)iterations, sizeof(double));
  char *payload = malloc((size_t)payload_size + 1);
  double total_start;
  double total_end;
  double total_us;
  double avg_us = 0.0;
  FILE *out = NULL;

  if (latencies == NULL || payload == NULL) {
    fprintf(stderr, "allocation failed\n");
    return 1;
  }

  memset(payload, 'x', (size_t)payload_size);
  payload[payload_size] = '\0';

  if (sqlite3_open(db_path, &db) != SQLITE_OK) {
    fprintf(stderr, "open failed: %s\n", sqlite3_errmsg(db));
    return 1;
  }

  exec_or_die(db, "PRAGMA temp_store=MEMORY;");
  exec_or_die(db, "PRAGMA foreign_keys=OFF;");
  exec_or_die(db, "PRAGMA wal_autocheckpoint=0;");
  {
    char pragma[128];
    snprintf(pragma, sizeof(pragma), "PRAGMA journal_mode=%s;", journal);
    exec_or_die(db, pragma);
    snprintf(pragma, sizeof(pragma), "PRAGMA synchronous=%s;", synchronous);
    exec_or_die(db, pragma);
  }
  exec_or_die(db, "CREATE TABLE IF NOT EXISTS commits (id INTEGER PRIMARY KEY, payload TEXT NOT NULL);");

  if (sqlite3_prepare_v2(db, "INSERT INTO commits(payload) VALUES (?1)", -1, &stmt, NULL) != SQLITE_OK) {
    fprintf(stderr, "prepare failed: %s\n", sqlite3_errmsg(db));
    return 1;
  }

  total_start = now_us();
  for (int i = 0; i < iterations; i += 1) {
    const double start = now_us();
    exec_or_die(db, "BEGIN IMMEDIATE;");
    sqlite3_bind_text(stmt, 1, payload, payload_size, SQLITE_TRANSIENT);
    if (sqlite3_step(stmt) != SQLITE_DONE) {
      fprintf(stderr, "step failed: %s\n", sqlite3_errmsg(db));
      return 1;
    }
    sqlite3_reset(stmt);
    sqlite3_clear_bindings(stmt);
    exec_or_die(db, "COMMIT;");
    latencies[i] = now_us() - start;
    avg_us += latencies[i];
  }
  total_end = now_us();
  total_us = total_end - total_start;
  avg_us /= (double)iterations;
  qsort(latencies, (size_t)iterations, sizeof(double), compare_double);

  out = fopen(raw_json, "w");
  if (out == NULL) {
    fprintf(stderr, "cannot open raw json path\n");
    return 1;
  }
  fprintf(out,
          "{\n"
          "  \"iterations\": %d,\n"
          "  \"payload_size\": %d,\n"
          "  \"journal_mode\": \"%s\",\n"
          "  \"synchronous\": \"%s\",\n"
          "  \"commit_count\": %d,\n"
          "  \"sync_call_count\": null,\n"
          "  \"total_seconds\": %.6f,\n"
          "  \"avg_commit_latency_us\": %.3f,\n"
          "  \"p50_commit_latency_us\": %.3f,\n"
          "  \"p95_commit_latency_us\": %.3f,\n"
          "  \"p99_commit_latency_us\": %.3f\n"
          "}\n",
          iterations,
          payload_size,
          journal,
          synchronous,
          iterations,
          total_us / 1000000.0,
          avg_us,
          percentile(latencies, iterations, 50.0),
          percentile(latencies, iterations, 95.0),
          percentile(latencies, iterations, 99.0));
  fclose(out);

  sqlite3_finalize(stmt);
  sqlite3_close(db);
  free(latencies);
  free(payload);
  return 0;
}
