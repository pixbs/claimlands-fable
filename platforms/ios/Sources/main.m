#import <UIKit/UIKit.h>

// Defined in crates/cl-app/src/platform.rs; runs the winit event loop and never returns.
extern void claimlands_main(void);

int main(int argc, char *argv[]) {
    @autoreleasepool {
        claimlands_main();
    }
    return 0;
}
