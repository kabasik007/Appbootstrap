package dev.appbootstrap.core.data

import dev.appbootstrap.core.model.Item
import dev.appbootstrap.core.model.ItemRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * Thread-safe demo adapter. Data is lost after process termination by design.
 * Replace with Room / DataStore only when product requirements demand persistence.
 */
class InMemoryItemRepository : ItemRepository {
    private val lock = Mutex()
    private var nextId = 1L
    private val mutableItems = MutableStateFlow<List<Item>>(emptyList())

    override val items: StateFlow<List<Item>> = mutableItems.asStateFlow()

    override suspend fun add(title: String) {
        val cleanTitle = title.trim()
        require(cleanTitle.isNotEmpty()) { "Title must not be blank" }
        lock.withLock {
            val newItem = Item(id = nextId++, title = cleanTitle)
            mutableItems.update { current -> current + newItem }
        }
    }
}
