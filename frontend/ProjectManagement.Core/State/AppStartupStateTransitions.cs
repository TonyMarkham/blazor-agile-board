namespace ProjectManagement.Core.State;

/// <summary>
/// Valid state transitions for the startup state machine.
/// </summary>
public static class AppStartupStateTransitions
{
    private static readonly Dictionary<AppStartupState, AppStartupState[]> ValidTransitions = new()
    {
        [AppStartupState.Initializing] = new[] {
            AppStartupState.WaitingForServer,
            AppStartupState.Ready, // Web mode
            AppStartupState.Error
        },
        [AppStartupState.WaitingForServer] = new[] {
            AppStartupState.CheckingIdentity,
            AppStartupState.Error
        },
        [AppStartupState.CheckingIdentity] = new[] {
            AppStartupState.NeedsRegistration,
            AppStartupState.ConnectingWebSocket,
            AppStartupState.Error
        },
        [AppStartupState.NeedsRegistration] = new[] {
            AppStartupState.ConnectingWebSocket,
            AppStartupState.Error
        },
        [AppStartupState.ConnectingWebSocket] = new[] {
            AppStartupState.Ready,
            AppStartupState.Error
        },
        [AppStartupState.Ready] = new[] {
            AppStartupState.Reconnecting,
            AppStartupState.Error
        },
        [AppStartupState.Reconnecting] = new[] {
            AppStartupState.Ready,
            AppStartupState.Error
        },
        [AppStartupState.Error] = new[] {
            AppStartupState.Initializing // Retry
        }
    };

    /// <summary>
    /// Validates if a state transition is allowed.
    /// </summary>
    public static bool CanTransition(AppStartupState from, AppStartupState to)
    {
        return ValidTransitions.TryGetValue(from, out var valid) && valid.Contains(to);
    }

    /// <summary>
    /// Transitions to new state, throwing if invalid.
    /// </summary>
    public static AppStartupState Transition(AppStartupState from, AppStartupState to)
    {
        if (!CanTransition(from, to))
        {
            throw new InvalidOperationException(
                $"Invalid state transition from {from} to {to}");
        }
        return to;
    }
}