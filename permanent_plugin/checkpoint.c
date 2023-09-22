#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>
#include <ctype.h>
#include <string.h>

static volatile uint8_t global_value;

int main(int argc, char** argv) {
	if (argc != 2) {
		fprintf(stderr, "usage: checkpoint <value>\n");
		return 1;
	}
    char *s = argv[1];
    uint8_t value;

    if (*s >= 'a' && *s <= 'z') {
        // string message (used to signal success)
        do {
            *s++ -= 32; // uppercase
        } while (*s >= 'a' && *s <= 'z');
        if (*s) {
            printf("expected only lowercase letters but found %c (%02x)\n", *s, *s);
            return 1;
        }
        printf("\nPERMANENT %s\n", argv[1]);
        return 0;
    } else if (isdigit(s[0])) {
        // pass number as value
        char *error = NULL;
        unsigned long value_long = strtoul(s, &error, 10);
        if (*error != '\0') {
            printf("expected a number but found %c (%02x)\n", *error, *error);
            return 1;
        } else if (value_long > 255) {
            printf("only checkpoint values up to 255 supported\n");
            return 1;
        }
        value = (uint8_t) value_long;
    } else {
        printf("invalid argument\n");
        return 1;
    }


    // key: ascii bytes "perm"
	asm __volatile__(
	"mov $0x6d726570, %%eax \t\n"
    "mov %1, %0 \t\n"
	: "=m" (global_value) /* output */
	: "r" (value) /* input */
	: "eax" /* clobbers */
	);

	return 0;
}
