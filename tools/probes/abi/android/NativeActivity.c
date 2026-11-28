#include <android/native_activity.h>

extern int oot_abi_driver_main(void);

__attribute__((visibility("default")))
void ANativeActivity_onCreate(ANativeActivity *activity, void *saved_state,
                              size_t saved_state_size) {
    (void)activity;
    (void)saved_state;
    (void)saved_state_size;
    (void)oot_abi_driver_main();
}
