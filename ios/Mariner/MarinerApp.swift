import SwiftUI

@main
struct MarinerApp: App {
    @State private var model = AppModel()

    init() {
        Images.configure()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .task { await model.open() }
        }
    }
}
