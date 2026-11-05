use leocard_protocol::{
    BlackjackCommand, DoudizhuCommand, GameCommand, MahjongCommand, McGuessCommand,
    QiGui523Command, ShengjiCommand, TexasCommand, UnoCommand,
};

pub(super) fn is_decision(command: &GameCommand) -> bool {
    match command {
        GameCommand::QiGui523(command) => matches!(
            command,
            QiGui523Command::PlayCards { .. } | QiGui523Command::Pass
        ),
        GameCommand::Texas(command) => matches!(command, TexasCommand::Act { .. }),
        GameCommand::Shengji(command) => !matches!(
            command,
            ShengjiCommand::SetAutoPlay { .. } | ShengjiCommand::UpdateRules { .. }
        ),
        GameCommand::Uno(command) => !matches!(
            command,
            UnoCommand::SetAutoPlay { .. }
                | UnoCommand::UpdateRules { .. }
                | UnoCommand::JumpIn { .. }
                | UnoCommand::CallUno
                | UnoCommand::ReportUno { .. }
        ),
        GameCommand::Mahjong(command) => !matches!(
            command,
            MahjongCommand::SetAutoPlay { .. }
                | MahjongCommand::UpdateRules { .. }
                | MahjongCommand::SetDeveloperHand { .. }
        ),
        GameCommand::McGuess(command) => {
            matches!(command, McGuessCommand::Guess { .. } | McGuessCommand::Skip)
        }
        GameCommand::Blackjack(command) => matches!(command, BlackjackCommand::Act { .. }),
        GameCommand::Doudizhu(command) => matches!(command, DoudizhuCommand::Act(_)),
    }
}
