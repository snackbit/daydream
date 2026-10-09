#define _GNU_SOURCE
#include <dlfcn.h>

/* Broadway exposes -1 when screen resolution is unset. Correct only that test
 * display sentinel; production binaries, styles and positive DPI are untouched. */
double gdk_screen_get_resolution(void *screen) {
    static double (*original)(void *);
    if (!original) original = dlsym(RTLD_NEXT, "gdk_screen_get_resolution");
    const double resolution = original ? original(screen) : -1.0;
    return resolution > 0.0 ? resolution : 96.0;
}
