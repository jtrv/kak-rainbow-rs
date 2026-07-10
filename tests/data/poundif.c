#include <stdio.h>

/* block comment with (brackets) [inside]
   spanning lines */
static const char *s = "string with ) and (";

#if 0
int disabled(int x) { return (x + 1); }
#else
int enabled(int x) { return (x * 2); }
#endif

#if 1
int also_enabled(void)
{
    char c = '(';
    int arr[3] = {1, 2, 3};
    // line comment with ) bracket
    return arr[0] + (arr[1] * arr[2]);
}
#endif

#if FEATURE
int maybe(void) { return (0); }
#endif

int main(void)
{
    printf("%d\n", also_enabled());
    return 0;
}
