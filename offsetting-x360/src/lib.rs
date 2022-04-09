use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use clap::{Subcommand, Parser};
use x_flipper_360;
use soiboy;
use soiboy::ComponentKind::Texture;
use soiboy::{SoiSoup, Str};
use x_flipper_360::{convert_to_dds, TextureHeader};

#[derive(Parser, Debug)]
pub struct X360Module {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Action {
    Extract(ExtractAction),
    ExtractCool(ExtractCoolAction),
    Repack,
    Ls(LsAction),
}

#[derive(Parser, Debug)]
pub(crate) struct ExtractAction {
    soi: PathBuf,
    toc: PathBuf,
    str: PathBuf,
}

#[derive(Parser, Debug)]
pub(crate) struct ExtractCoolAction {
    soi: PathBuf,
    toc: PathBuf,
    stream: PathBuf,
}

#[derive(Parser, Debug)]
pub(crate) struct LsAction {
    soi: PathBuf,
    toc: PathBuf,
}

impl X360Module {
    pub fn execute(&self) {
        match &self.action {
            Action::Extract(action) => action.execute().unwrap(),
            Action::ExtractCool(action) => action.execute().unwrap(),
            Action::Repack => {},
            Action::Ls(action) => action.execute().unwrap(),
        }
    }
}

impl ExtractAction {
    fn execute(&self) -> anyhow::Result<()> {
        let soi = soiboy::Soi::<x_flipper_360::TextureHeader>::read(self.soi.as_path())?;
        let toc = soiboy::Toc::read(self.toc.as_path())?;
        let mut str = soiboy::Str::read(self.str.as_path())?;
        // str.read_section_data(toc.sections.get(0)?);
        for (id, section) in toc.sections.iter().enumerate() {
            let data = str.read_section_data(section)?;

            for component in data.uncached {
                if component.kind != Texture { continue; }
                let header = match soi.find_texture_header(id as u32, component.id) {
                    None => match toc.find_ids(component.instance_id) {
                        None => panic!("Instance ID not found."),
                        Some((section_id, component_id)) => match soi.find_texture_header(section_id, component_id) {
                            None => panic!("Component ID not found."),
                            Some(header) => header
                        }
                    }
                    Some(header) => header
                };

                println!("{} {:?}", component.path, header.metadata());
                let metadata = header.metadata();
                let texture_size = x_flipper_360::TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

                let config = x_flipper_360::Config {
                    width: (texture_size.width() + 1) as u32,
                    height: (texture_size.height() + 1) as u32,
                    depth: Some(1),
                    pitch: metadata.pitch() as u32,
                    tiled: metadata.tiled(),
                    packed_mips: metadata.packed_mips(),
                    format: match metadata.format() {
                        x_flipper_360::TextureFormat::Dxt1 => x_flipper_360::Format::Dxt1,
                        x_flipper_360::TextureFormat::Dxt4_5 => x_flipper_360::Format::Dxt5,
                        x_flipper_360::TextureFormat::_8_8_8_8 => x_flipper_360::Format::RGBA8,
                        // _ => panic!("{:?}", metadata.format()),
                        _ => continue // todo,
                    },
                    mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
                    base_address: metadata.base_address() as u32,
                    mip_address: metadata.mip_address() as u32
                };

                let path = PathBuf::from(format!("data/out/{}.dds", component.path.replace("\\", "/")));
                create_dir_all(&path.parent().unwrap());
                let mut file = File::create(&path)?;
                let path2 = path.with_extension(".orgi");
                let mut file2 = File::create(path2)?;
                file2.write_all(&component.data)?;

                x_flipper_360::convert_to_dds(&config, &component.data, &mut file)?;
            }

            for component in data.cached {
                if component.kind != Texture { continue; }
                let header = match soi.find_texture_header(id as u32, component.id) {
                    None => match toc.find_ids(component.instance_id) {
                        None => panic!("til"),
                        Some((section_id, component_id)) => match soi.find_texture_header(section_id, component_id) {
                            None => panic!("klemens"),
                            Some(header) => header
                        }
                    }
                    Some(header) => header
                };

                println!("{} {:?}", component.path, header.metadata());
            }
        }
        Ok(())
    }
}

impl ExtractCoolAction {
    fn execute(&self) -> anyhow::Result<()> {

        let soi_soup =
            SoiSoup::cook(self.toc.as_path(), self.soi.as_path())?;
        let mut stream = Str::read(self.stream.as_path())?;

        for (section_id, section) in soi_soup.find_sections().iter().enumerate() {
            let section_data = stream.read_section_data(section)?;
            for uncached_component in section_data.uncached {
                let texture_header: &TextureHeader = soi_soup.find_texture_header(section_id as u32,
                                                                  uncached_component.id,
                                                                  uncached_component.instance_id).unwrap();
                let metadata = texture_header.metadata();

                let texture_size = x_flipper_360::TextureSize2D::from_bytes(metadata.texture_size().to_le_bytes());

                let path = PathBuf::from(format!("data/out/{}.dds", uncached_component.path));
                create_dir_all(path.parent().unwrap())?;
                let mut file = File::create(path)?;

                let config = x_flipper_360::Config {
                    width: (texture_size.width() + 1) as u32,
                    height: (texture_size.height() + 1) as u32,
                    depth: Some(1),
                    pitch: metadata.pitch() as u32,
                    tiled: metadata.tiled(),
                    packed_mips: metadata.packed_mips(),
                    format: match metadata.format() {
                        x_flipper_360::TextureFormat::Dxt1 => x_flipper_360::Format::Dxt1,
                        x_flipper_360::TextureFormat::Dxt4_5 => x_flipper_360::Format::Dxt5,
                        x_flipper_360::TextureFormat::_8_8_8_8 => x_flipper_360::Format::RGBA8,
                        // _ => panic!("{:?}", metadata.format()),
                        _ => continue // todo,
                    },
                    mipmap_levels: Some(1.max(metadata.max_mip_level() - metadata.min_mip_level()) as u32),
                    base_address: metadata.base_address() as u32,
                    mip_address: metadata.mip_address() as u32
                };

                convert_to_dds(&config, &uncached_component.data, &mut file)?;
            }
        }
        Ok(())
    }
}

impl LsAction {
    fn execute(&self) -> anyhow::Result<()> {
        let soi_soup =
            SoiSoup::<TextureHeader>::cook(self.toc.as_path(), self.soi.as_path())?;

        for (section_id, _, component_header) in soi_soup.find_components() {
            println!("Section_Id: {}, FileName: {}", section_id, component_header.path());
        }
        Ok(())
    }
}
