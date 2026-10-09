package dev.appbootstrap.core.model

import kotlinx.coroutines.flow.StateFlow

/** Stable contract: UI never knows whether items come from memory, Room or HTTP. */
interface ItemRepository {
    val items: StateFlow<List<Item>>

    suspend fun add(title: String)
}
