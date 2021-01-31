use log::warn;

macro_rules! gen_str_buff {
	($(#[$attr:meta] => $type:ident, $size:expr),*) => { $(
	    #[$attr]
	    #[derive(Clone, Copy)]
		pub struct $type {
			pub len: usize,
			pub buff: [u8; $size],
		}

		impl $type {
			pub fn new() -> Self {
				Self {
					len: 0,
					buff: [0; $size]
				}
			}

			pub fn to_opt(self) -> Option<String> {
				if self.len == 0 {
					return None;
				}

				Some(String::from(self))
			}

			pub fn set<T: AsRef<str>>(&mut self, value: T) -> Self {
				let bytes = value.as_ref().as_bytes();

				let mut len = bytes.len();

				if len > $size{
					len = $size;
				}

				self.len = len;

				self.buff[0..len].copy_from_slice(&bytes[0..len]);

				*self
			}

			pub fn set_trim<T: AsRef<str>>(&mut self, value: T, trim: char) -> Self {
				let value_ref = value.as_ref();
				let value_string = value_ref.to_string();
				let bytes = value_ref.as_bytes();

				let mut total_len = value_string.len();

			    value_ref.to_string().chars().rev().take_while(|c| {
			        total_len -= 1;
			        ( *c != trim ) || ( total_len > $size )
			    }).collect::<String>().len();

			    self.len = total_len;

			    self.buff[0..total_len].copy_from_slice(&bytes[0..total_len]);

				*self
			}
		}

		impl std::convert::From<&str> for $type {
			fn from(value: &str) -> Self {
				let bytes = value.as_bytes();

				let mut len = bytes.len();

				if len > $size{
					len = $size;
				}

				let mut buff = $type::new();

                buff.len = len;
				buff.buff[0..len].copy_from_slice(&bytes[0..len]);

				buff
			}
		}

		impl std::convert::From<String> for $type {
			fn from(value: String) -> Self {
				Self::from(value.as_str())
			}
		}

		impl std::convert::From<$type> for String {
			fn from(buff: $type) -> String {
				std::str::from_utf8(&buff.buff[0..buff.len]).unwrap().to_string()
			}
		}

		impl std::fmt::Display for $type {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		        write!(f, "{}", String::from(*self))
		    }
		}

		impl std::fmt::Debug for $type {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		        write!(f, "[{}[{}]: '{}']", stringify!($type), self.len, String::from(*self))
		    }
		}
	)* }
}

const UCI_MAX_LENGTH: usize = 5;
const UCI_TYPICAL_LENGTH: usize = 4;
const MAX_PV_MOVES: usize = 2;
const PV_BUFF_SIZE: usize = MAX_PV_MOVES * (UCI_TYPICAL_LENGTH + 1);

gen_str_buff!(
/// UciBuff
=> UciBuff, UCI_MAX_LENGTH,
/// PvBuff
=> PvBuff, PV_BUFF_SIZE
);

/// score
#[derive(Debug, Clone, Copy)]
pub enum Score {
    Cp(i32),
    Mate(i32),
}

/// analysis info
#[derive(Debug, Clone, Copy)]
pub struct AnalysisInfo {
    /// best move
    bestmove: UciBuff,
    /// ponder
    ponder: UciBuff,
    /// pv
    pv: PvBuff,
    /// multipv
    pub multipv: usize,
    /// depth
    pub depth: usize,
    /// seldepth
    pub seldepth: usize,
    /// tbhits
    pub tbhits: u64,
    /// nodes
    pub nodes: u64,
    /// time
    pub time: usize,
    /// nodes per second
    pub nps: u64,
    /// score ( centipawns or mate )
    pub score: Score,
}

/// parsing state
#[derive(Debug)]
#[allow(dead_code)]
// TODO: make this pub(crate)
pub enum ParsingState {
    Info,
    Key,
    Unknown,
    Multipv,
    Depth,
    Seldepth,
    Tbhits,
    Nodes,
    Time,
    Nps,
    Score,
    ScoreCp,
    ScoreMate,
    PvBestmove,
    PvPonder,
    PvRest,
}

/// analysis info implementation
impl AnalysisInfo {
    /// create new analysis info
    pub fn new() -> Self {
        Self {
            bestmove: UciBuff::new(),
            ponder: UciBuff::new(),
            pv: PvBuff::new(),
            multipv: 0,
            depth: 0,
            seldepth: 0,
            tbhits: 0,
            nodes: 0,
            time: 0,
            nps: 0,
            score: Score::Cp(0),
        }
    }

    // get bestmove
    pub fn bestmove(self) -> Option<String> {
        self.bestmove.to_opt()
    }

    // get ponder
    pub fn ponder(self) -> Option<String> {
        self.ponder.to_opt()
    }

    // get pv
    pub fn pv(self) -> Option<String> {
        self.pv.to_opt()
    }

    /// parse info string
    pub fn parse<T: std::convert::AsRef<str>>(&mut self, info: T) {
        let info = info.as_ref();
        let mut ps = ParsingState::Info;
        let mut pv_buff = String::new();
        let mut pv_on = false;

        for token in info.split(" ") {
            match ps {
                ParsingState::Info => {
                    match token {
                        "info" => ps = ParsingState::Key,
                        _ => {
                            // not an info
                            return;
                        }
                    }
                }
                ParsingState::Key => {
                    if token == "string" {
                        // anything starting with 'info string' is not analysis info
                        // occuring later in key position 'string' is not a valid analysis info token
                        return;
                    }

                    ps = match token {
                        "multipv" => ParsingState::Multipv,
                        "depth" => ParsingState::Depth,
                        "seldepth" => ParsingState::Seldepth,
                        "tbhits" => ParsingState::Tbhits,
                        "nodes" => ParsingState::Nodes,
                        "time" => ParsingState::Time,
                        "nps" => ParsingState::Nps,
                        "score" => ParsingState::Score,
                        "pv" => ParsingState::PvBestmove,
                        _ => ParsingState::Unknown,
                    }
                }
                ParsingState::Score => match token {
                    "cp" => ps = ParsingState::ScoreCp,
                    "mate" => ps = ParsingState::ScoreMate,
                    _ => {
                        warn!("invalid score specifier {}", token);

                        return;
                    }
                },
                ParsingState::Unknown => {
                    // ignore this token and hope for the best
                    ps = ParsingState::Key
                }
                _ => {
                    match ps {
                        ParsingState::Multipv => match token.parse::<usize>() {
                            Ok(multipv) => self.multipv = multipv,
                            _ => {
                                warn!("could not parse multipv from {}", token)
                            }
                        },
                        ParsingState::Depth => match token.parse::<usize>() {
                            Ok(depth) => self.depth = depth,
                            _ => {
                                warn!("could not parse depth from {}", token)
                            }
                        },
                        ParsingState::Seldepth => match token.parse::<usize>() {
                            Ok(seldepth) => self.seldepth = seldepth,
                            _ => {
                                warn!("could not parse seldepth from {}", token)
                            }
                        },
                        ParsingState::Tbhits => match token.parse::<u64>() {
                            Ok(tbhits) => self.tbhits = tbhits,
                            _ => {
                                warn!("could not parse tbhits from {}", token)
                            }
                        },
                        ParsingState::Nodes => match token.parse::<u64>() {
                            Ok(nodes) => self.nodes = nodes,
                            _ => {
                                warn!("could not parse nodes from {}", token)
                            }
                        },
                        ParsingState::Nps => match token.parse::<u64>() {
                            Ok(nps) => self.nps = nps,
                            _ => {
                                warn!("could not parse nps from {}", token)
                            }
                        },
                        ParsingState::Time => match token.parse::<usize>() {
                            Ok(time) => self.time = time,
                            _ => {
                                warn!("could not parse time from {}", token)
                            }
                        },
                        ParsingState::ScoreCp => match token.parse::<i32>() {
                            Ok(score_cp) => self.score = Score::Cp(score_cp),
                            _ => {
                                warn!("could not parse score cp from {}", token)
                            }
                        },
                        ParsingState::ScoreMate => match token.parse::<i32>() {
                            Ok(score_mate) => self.score = Score::Mate(score_mate),
                            _ => {
                                warn!("could not parse score mate from {}", token)
                            }
                        },
                        ParsingState::PvBestmove => {
                            pv_buff = pv_buff + token;

                            self.bestmove = UciBuff::from(token);

                            pv_on = true;

                            ps = ParsingState::PvPonder
                        }
                        ParsingState::PvPonder => {
                            pv_buff = pv_buff + " " + token;

                            self.ponder = UciBuff::from(token);

                            ps = ParsingState::PvRest
                        }
                        ParsingState::PvRest => pv_buff = pv_buff + " " + token,
                        _ => {
                            // should not happen
                        }
                    }

                    if !pv_on {
                        ps = ParsingState::Key;
                    }
                }
            }
        }

        self.pv = PvBuff::from(pv_buff);
    }
}
