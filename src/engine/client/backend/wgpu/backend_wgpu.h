#include <engine/client/backend/backend_base.h>
#include <engine/client/backend/backend_wgpu.h>

class CCommandProcessorFragment_WGPU : public CCommandProcessorFragment_GLBase
{
	bool GetPresentedImageData(uint32_t &Width, uint32_t &Height, CImageInfo::EImageFormat &Format, std::vector<uint8_t> &vDstData) override { return false; }
	ERunCommandReturnTypes RunCommand(const CCommandBuffer::SCommand *pBaseCommand) override;
	bool Cmd_Init(const SCommand_Init *pCommand);
	virtual void Cmd_Texture_Create(const CCommandBuffer::SCommand_Texture_Create *pCommand);
	virtual void Cmd_Texture_Destroy(const CCommandBuffer::SCommand_Texture_Destroy *pCommand);
	virtual void Cmd_TextTextures_Create(const CCommandBuffer::SCommand_TextTextures_Create *pCommand);
	virtual void Cmd_TextTextures_Destroy(const CCommandBuffer::SCommand_TextTextures_Destroy *pCommand);
	virtual void Cmd_TextTexture_Update(const CCommandBuffer::SCommand_TextTexture_Update *pCommand);

	rust::Box<RustWgpuBackend> m_Rust = init_rust_wgpu_backend();
};

CCommandProcessorFragment_GLBase *CreateWGPUCommandProcessorFragment();
