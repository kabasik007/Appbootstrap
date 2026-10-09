package dev.appbootstrap.android

import android.app.Application
import dev.appbootstrap.core.data.InMemoryItemRepository
import dev.appbootstrap.core.model.ItemRepository

/** Process-wide composition root. Swap data implementation without changing UI. */
class StarterApplication : Application() {
    val container: AppContainer by lazy { AppContainer() }
}

class AppContainer {
    val itemRepository: ItemRepository = InMemoryItemRepository()
}
