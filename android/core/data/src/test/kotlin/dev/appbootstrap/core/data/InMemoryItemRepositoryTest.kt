package dev.appbootstrap.core.data

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class InMemoryItemRepositoryTest {
    @Test fun addingItemsEmitsInOrderWithUniqueIds() = runBlocking {
        val repository = InMemoryItemRepository()
        repository.add(" first ")
        repository.add("second")
        assertEquals(listOf("first", "second"), repository.items.value.map { it.title })
        assertEquals(2, repository.items.value.map { it.id }.distinct().size)
    }

    @Test fun blankTitlesAreRejected() {
        val repository = InMemoryItemRepository()
        assertThrows(IllegalArgumentException::class.java) {
            runBlocking { repository.add("  ") }
        }
        assertEquals(emptyList<Any>(), repository.items.value)
    }
}
