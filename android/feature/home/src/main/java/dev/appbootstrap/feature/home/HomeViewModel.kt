package dev.appbootstrap.feature.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import dev.appbootstrap.core.model.Item
import dev.appbootstrap.core.model.ItemRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class HomeUiState(
    val items: List<Item> = emptyList(),
    val draft: String = "",
)

class HomeViewModel(private val repository: ItemRepository) : ViewModel() {
    private val draft = MutableStateFlow("")

    val uiState: StateFlow<HomeUiState> = combine(repository.items, draft) { items, text ->
        HomeUiState(items = items, draft = text)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), HomeUiState())

    fun onDraftChanged(text: String) {
        draft.value = text.take(200)
    }

    fun onAddClicked() {
        val submittedTitle = draft.value.trim()
        if (submittedTitle.isEmpty()) return
        draft.value = ""
        viewModelScope.launch { repository.add(submittedTitle) }
    }

    companion object {
        fun factory(repository: ItemRepository): ViewModelProvider.Factory = viewModelFactory {
            initializer { HomeViewModel(repository) }
        }
    }
}
