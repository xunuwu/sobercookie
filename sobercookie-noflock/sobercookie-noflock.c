#include <linux/errno.h>
#include <seccomp.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char **argv) {
  if (argc < 2) {
    printf("you need to provide 1 argument\n");
    return EXIT_FAILURE;
  }

  if (argc > 2) {
    fprintf(stderr, "arguments after the second one will be ignored\n");
    fprintf(stderr, "you should double check your quoting\n");
  }

  scmp_filter_ctx filter = seccomp_init(SCMP_ACT_ALLOW);
  seccomp_rule_add(filter, SCMP_ACT_ERRNO(EBADF), SCMP_SYS(flock), 0);
  seccomp_load(filter);

  execv("/bin/sh", (char *[]){"/bin/sh", "-c", argv[1], NULL});
}
