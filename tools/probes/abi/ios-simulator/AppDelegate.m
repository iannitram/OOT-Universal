#import <UIKit/UIKit.h>

extern int oot_abi_driver_main(void);

@interface AppDelegate : UIResponder <UIApplicationDelegate>
@property(nonatomic, strong) UIWindow *window;
@end

@implementation AppDelegate
- (BOOL)application:(UIApplication *)application
        didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    (void)application;
    (void)launchOptions;

    int result = oot_abi_driver_main();
    NSString *message = result == 0 ? @"C/Rust ABI checks passed"
                                    : [NSString stringWithFormat:@"C/Rust ABI checks failed (%d)", result];
    NSLog(@"%@", message);

    self.window = [[UIWindow alloc] initWithFrame:UIScreen.mainScreen.bounds];
    UIViewController *controller = [UIViewController new];
    UILabel *label = [UILabel new];
    label.text = message;
    label.textAlignment = NSTextAlignmentCenter;
    label.numberOfLines = 0;
    label.frame = controller.view.bounds;
    label.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    [controller.view addSubview:label];
    self.window.rootViewController = controller;
    [self.window makeKeyAndVisible];
    return YES;
}
@end

int main(int argc, char *argv[]) {
    @autoreleasepool {
        return UIApplicationMain(argc, argv, nil, NSStringFromClass(AppDelegate.class));
    }
}
