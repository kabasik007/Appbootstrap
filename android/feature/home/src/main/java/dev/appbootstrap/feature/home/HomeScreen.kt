package dev.appbootstrap.feature.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle

@Composable
fun HomeScreen(viewModel: HomeViewModel) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    HomeContent(
        state = state,
        onDraftChanged = viewModel::onDraftChanged,
        onAddClicked = viewModel::onAddClicked,
    )
}

/** Stateless and preview/test friendly UI: all input flows through explicit events. */
@Composable
fun HomeContent(
    state: HomeUiState,
    onDraftChanged: (String) -> Unit,
    onAddClicked: () -> Unit,
) {
    Scaffold(topBar = { TopAppBar(title = { Text(stringResource(R.string.home_title)) }) }) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(stringResource(R.string.home_description))
            OutlinedTextField(
                value = state.draft,
                onValueChange = onDraftChanged,
                label = { Text(stringResource(R.string.home_item_label)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Button(onClick = onAddClicked, enabled = state.draft.isNotBlank()) {
                Text(stringResource(R.string.home_add))
            }
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                contentPadding = PaddingValues(vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(state.items, key = { it.id }) { item ->
                    Text(text = item.title)
                }
            }
        }
    }
}
