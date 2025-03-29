#include "base/system.h"

#include <engine/client/backend/backend_base.h>
#include <engine/client/backend/backend_wgpu.h>

class CCommandProcessorFragment_WGPU : public CCommandProcessorFragment_GLBase
{
	bool GetPresentedImageData(uint32_t &Width, uint32_t &Height, CImageInfo::EImageFormat &Format, std::vector<uint8_t> &vDstData) override { return false; }
	ERunCommandReturnTypes RunCommand(const CCommandBuffer::SCommand *pBaseCommand) override;
	RustWgpuBackend *Rust()
	{
		return &*(*m_Rust);
	}
	bool Cmd_PreInit(const SCommand_PreInit *pCommand);
	bool Cmd_Init(const SCommand_Init *pCommand);
	virtual void Cmd_UpdateViewport(const CCommandBuffer::SCommand_Update_Viewport *pCommand);
	virtual void Cmd_Swap(const CCommandBuffer::SCommand_Swap *pCommand);
	virtual void Cmd_Texture_Create(const CCommandBuffer::SCommand_Texture_Create *pCommand);
	virtual void Cmd_Texture_Destroy(const CCommandBuffer::SCommand_Texture_Destroy *pCommand);
	virtual void Cmd_TextTextures_Create(const CCommandBuffer::SCommand_TextTextures_Create *pCommand);
	virtual void Cmd_TextTextures_Destroy(const CCommandBuffer::SCommand_TextTextures_Destroy *pCommand);
	virtual void Cmd_TextTexture_Update(const CCommandBuffer::SCommand_TextTexture_Update *pCommand);
	virtual void Cmd_Clear(const CCommandBuffer::SCommand_Clear *pCommand);
	virtual void Cmd_Render(const CCommandBuffer::SCommand_Render *pCommand);

	std::optional<rust::Box<RustWgpuBackend>> m_Rust;

public:
	CCommandProcessorFragment_WGPU(int WgpuBackendType)
	{
		m_Rust = init_rust_wgpu_backend(WgpuBackendType);
		dbg_assert(Rust() != nullptr, "Silent WGPU backend initialization failure");
	}
};

CCommandProcessorFragment_GLBase *CreateWGPUCommandProcessorFragment(int WgpuBackendType);
