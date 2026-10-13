/* MIT. Measure the executed child, without inheriting Python harness pages. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
int main(int argc, char **argv) {
    if (argc < 2) return 2;
    pid_t pid = fork();
    if (pid < 0) return 1;
    if (pid == 0) {
        int fd = open("/dev/null", O_WRONLY);
        if (fd < 0) _exit(127);
        dup2(fd, 1); dup2(fd, 2); close(fd);
        execvp(argv[1], argv + 1); _exit(127);
    }
    int status; struct rusage usage;
    if (wait4(pid, &status, 0, &usage) < 0) return 1;
    if (!WIFEXITED(status) || WEXITSTATUS(status)) return 1;
    printf("%ld\n", usage.ru_maxrss);
    return 0;
}
