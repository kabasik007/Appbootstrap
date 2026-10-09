package dev.appbootstrap.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.remember
import androidx.lifecycle.viewmodel.compose.viewModel
import dev.appbootstrap.feature.home.HomeScreen
import dev.appbootstrap.feature.home.HomeViewModel

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val repository = (application as StarterApplication).container.itemRepository

        setContent {
            MaterialTheme {
                Surface {
                    val factory = remember(repository) { HomeViewModel.factory(repository) }
                    val homeViewModel: HomeViewModel = viewModel(factory = factory)
                    HomeScreen(viewModel = homeViewModel)
                }
            }
        }
    }
}
